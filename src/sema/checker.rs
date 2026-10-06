//! Semantic traversal and construct-specific validation.

use super::*;

impl Checker<'_> {
    fn error(&mut self, error: TtError) {
        self.errors.push(error);
    }

    pub(super) fn visit_program(&mut self, program: &Program, ctx: Ctx, place: Place) {
        crate::stack::grow(|| self.visit_program_segments(program, ctx, place));
    }

    fn visit_program_segments(&mut self, program: &Program, ctx: Ctx, place: Place) {
        for error in &program.malformed {
            self.error(error.clone());
        }
        // A stray `|>` or `if let` cannot be passed through: neither is
        // valid TypeScript, so the output self-check would fail without a
        // position. Report them as tt errors here instead (error-layering
        // contract) — all of them, not the first.
        for &off in &program.stray_pipes {
            self.error(
                TtError::span(
                    off,
                    off + "|>".len(),
                    "pipeline: `|>` could not be parsed here".to_string(),
                )
                .code(DiagnosticCode::StrayPipe)
                .help("a step is an expression — parenthesize a ternary or an arrow function"),
            );
        }
        for stray in &program.stray_if_lets {
            let (message, help) = match stray.kind {
                crate::ast::StrayIfLetKind::Head => (
                    "`if let` could not be parsed here",
                    "the pattern parens are mandatory, and the `else` must be a block or \
                     another `if let`",
                ),
                crate::ast::StrayIfLetKind::ElseContinuation => (
                    "the `else` of an `if let` must be a block or another `if let`",
                    "put a plain `if (...)` inside an `else { ... }` block",
                ),
            };
            self.error(
                TtError::span(stray.span.start, stray.span.end, message.to_string())
                    .code(DiagnosticCode::StrayIfLet)
                    .help(help),
            );
        }
        for segment in &program.segments {
            match segment {
                Segment::Verbatim(_) | Segment::TtImport(_) | Segment::ValModifier(_) => {}
                Segment::Variant(decl) => self.check_variant(decl),
                Segment::Match(expr) => self.check_match(expr, place),
                Segment::TupleMatch(expr) => self.check_tuple_match(expr, place),
                Segment::Try(stmt) => self.check_try(stmt, place),
                Segment::TryExpr(expr) => self.check_try_expr(expr, place),
                Segment::LetElse(stmt) => self.check_let_else(stmt, place),
                Segment::IfLet(stmt) => self.check_if_let(stmt, ctx, place),
                Segment::ResultBlock(block) => self.check_result_block(block),
                Segment::Pipe(pipe) => {
                    for step in &pipe.steps {
                        if step.kind == PipeStepKind::Missing {
                            self.error(
                                TtError::span(
                                    step.span.start - "|>".len(),
                                    step.span.start,
                                    "pipeline: `|>` has no step".to_string(),
                                )
                                .code(DiagnosticCode::MissingPipelineStep)
                                .help("write the step after `|>`, or remove the `|>`"),
                            );
                        }
                    }
                    // A `flow` composition has no value to chain a method
                    // onto until its first function has produced one, so
                    // its first step must be an ordinary function step.
                    if pipe.head.is_none()
                        && let Some(first) = pipe.steps.first()
                        && matches!(first.kind, PipeStepKind::Postfix { .. })
                    {
                        self.error(
                            TtError::span(
                                first.span.start,
                                first.span.end,
                                "`flow`: the first step cannot be a method step — it is the \
                                 composed function's input, so it must be a function"
                                    .to_string(),
                            )
                            .code(DiagnosticCode::FlowFirstStepMethod)
                            .help(
                                "write the step as a function — \
                                 `flow |> ((s: string) => s.trim()) |> ...`",
                            ),
                        );
                    }
                    if pipe.head.is_none()
                        && let Some(first) = pipe.steps.first()
                        && matches!(first.kind, PipeStepKind::Call)
                        && crate::program_syntax::source_member_callee(
                            self.source,
                            crate::hir::Span {
                                start: first.span.start,
                                end: first.span.end,
                            },
                            self.source_kind,
                        )
                        .is_some_and(|member| member.optional)
                    {
                        self.error(
                            TtError::span(
                                first.span.start,
                                first.span.end,
                                "`flow`: the first step cannot be an optional-chain step — it is \
                                 the composed function's input, so it must be a function"
                                    .to_string(),
                            )
                            .code(DiagnosticCode::FlowFirstStepMethod)
                            .help(
                                "write the step as a function — \
                                 `flow |> ((n: number) => o?.m(n)) |> ...`",
                            ),
                        );
                    }
                    if pipe.head_kind == PipeHeadKind::BareSuper
                        && pipe.steps.first().is_some_and(|step| {
                            matches!(step.kind, PipeStepKind::Postfix { optional: true })
                        })
                    {
                        self.error(
                            TtError::span(
                                pipe.head_span.start,
                                pipe.head_span.end,
                                "pipeline: `super` cannot be an optional-chain receiver"
                                    .to_string(),
                            )
                            .code(DiagnosticCode::InvalidOptionalReceiver)
                            .owner(
                                pipe.head_span.start,
                                pipe.steps
                                    .last()
                                    .map_or(pipe.head_span.end, |step| step.span.end),
                            )
                            .help("access a concrete `super.member` before the optional step"),
                        );
                    }
                    // Head and steps are expressions — `try` inside them is
                    // rejected for the same reason as inside a match.
                    if let Some(head) = &pipe.head {
                        self.visit_program(head, Ctx::Expr, place);
                    }
                    for step in &pipe.steps {
                        self.visit_program(&step.body, Ctx::Expr, place.isolated());
                    }
                }
                Segment::Template(template) => {
                    for chunk in &template.chunks {
                        if let TemplateChunk::Interp(interp) = chunk {
                            self.visit_program(interp, Ctx::Expr, place.isolated());
                        }
                    }
                }
            }
        }
    }

    /// `try` placement is a **flow** fact, not a nesting rule. A statement
    /// `try` leaves its nearest Result scope (`docs/design/try-result-scopes.md`
    /// §4.2): a `result` block whose body holds it with no function written
    /// in between, otherwise the innermost function-like boundary around it.
    /// An isolated value region (a match arm, a pipeline step, an
    /// interpolation) is not a boundary: its `try` keeps the function
    /// target, and only a `try` whose nearest scope is a `result` block
    /// outside the region crosses it (§4.6). The target must then be able
    /// to return the `Err`: an ordinary function can; a constructor, a
    /// generator, class code outside a method, and a module's top level
    /// cannot. Inside a template literal, which the file's token stream
    /// holds as one token, the interpolation's own token stream is asked
    /// ([`crate::flow::FunctionTargets::at_offset`]), so a generator written
    /// there is the target as it is anywhere else.
    fn check_try(&mut self, stmt: &TryStmt, place: Place) {
        let function_target = match place {
            Place::ResultRegion if !stmt.in_function => {
                self.visit_program(&stmt.expr, Ctx::Expr, Place::ValueRegion);
                return;
            }
            Place::ResultValueRegion if !stmt.in_function => {
                self.error(
                    TtError::span(
                        stmt.span.start,
                        stmt.span.end,
                        "`try` crosses an isolated value region whose exits cannot target the enclosing `result` block".to_string(),
                    )
                    .code(DiagnosticCode::TryCrossesValueRegion)
                    .help("extract the affected expression into a nested function when doing so preserves its captures and evaluation order"),
                );
                self.visit_program(&stmt.expr, Ctx::Expr, Place::ValueRegion);
                return;
            }
            _ => self.function_targets().at_offset(stmt.span.start),
        };
        let (message, help) = match function_target {
            Some(crate::flow::FunctionTarget::Ordinary) => {
                self.visit_program(&stmt.expr, Ctx::Expr, Place::ValueRegion);
                return;
            }
            Some(crate::flow::FunctionTarget::Setter) => (
                "`try` cannot be used in a setter — a setter's return value is discarded, so its \
                 `Err` propagation could not reach the caller"
                    .to_string(),
                "move the propagation into an ordinary function, or handle the Result explicitly",
            ),
            Some(
                crate::flow::FunctionTarget::Constructor | crate::flow::FunctionTarget::Generator,
            ) => (
                "`try` cannot be used in a constructor or generator — its `Err` propagation \
                 requires an ordinary function return"
                    .to_string(),
                "move the propagation into an ordinary function, or handle the Result explicitly",
            ),
            Some(
                boundary @ (crate::flow::FunctionTarget::StaticBlock
                | crate::flow::FunctionTarget::ClassElement),
            ) => {
                let owner = if boundary == crate::flow::FunctionTarget::StaticBlock {
                    "a class static block"
                } else {
                    "a class field initializer or computed member name"
                };
                (
                    format!(
                        "`try` cannot be used in {owner} — it has no enclosing function failure \
                         edge for its `Err` propagation"
                    ),
                    "move the propagation into an ordinary function, or handle the Result explicitly",
                )
            }
            None => {
                let (message, help) = crate::diagnostics::TRY_OUTSIDE_FUNCTION;
                (message.to_string(), help)
            }
        };
        self.error(
            TtError::span(stmt.span.start, stmt.span.end, message)
                .code(DiagnosticCode::TryPlacement)
                .help(help),
        );
        self.visit_program(&stmt.expr, Ctx::Expr, Place::ValueRegion);
    }

    fn check_try_expr(&mut self, expr: &crate::ast::TryExpr, place: Place) {
        // Ordinary expression propagation is judged after the SWC host owner
        // and evaluation protocol are known. A result block is the one
        // surface-owned boundary: its direct propagation targets that region.
        if place == Place::ResultValueRegion && self.crosses_value_region(expr.span.start) {
            self.error(
                TtError::span(
                    expr.span.start,
                    expr.span.end,
                    "`try` crosses an isolated value region whose exits cannot target the enclosing `result` block".to_string(),
                )
                .code(DiagnosticCode::TryCrossesValueRegion)
                .help("extract the affected expression into a nested function when doing so preserves its captures and evaluation order"),
            );
        }
        self.visit_program(&expr.expr, Ctx::Expr, Place::ValueRegion);
    }

    fn crosses_value_region(&self, at: usize) -> bool {
        let Some(&result) = self.result_blocks.last() else {
            return true;
        };
        self.function_targets()
            .boundary_at_offset(at)
            .is_none_or(|boundary| boundary < result)
    }

    fn function_targets(&self) -> &crate::flow::FunctionTargets {
        self.function_targets.get_or_init(|| {
            crate::flow::FunctionTargets::new(self.tokens, &|tokens| {
                self.semantic.hir.match_owned_tokens(tokens)
            })
        })
    }

    /// let-else placement is the same flow fact as `try`'s, except the
    /// module's top level is fine: the lowering emits no `return` of its
    /// own (a `throw`-diverging `else` is valid anywhere), so only
    /// [`Place::ValueRegion`] regions — where the `else`'s exits would leave the
    /// construct's value boundary — need a function written in the region.
    fn check_let_else(&mut self, stmt: &LetElseStmt, place: Place) {
        if matches!(place, Place::ValueRegion | Place::ResultValueRegion) && !stmt.in_function {
            self.error(
                TtError::span(
                    stmt.head_span.start,
                    stmt.head_span.end,
                    "let-else cannot be used here — its `else` block's exit (`return`, \
                     `break`, `continue`) would complete this construct's value instead of \
                     leaving the enclosing function"
                        .to_string(),
                )
                .code(DiagnosticCode::LetElsePlacement)
                .help(
                    "extract the logic into a function (a let-else inside a function \
                     written here is fine), or `match` on the value instead",
                ),
            );
        }
        if !stmt.diverges {
            self.error(
                TtError::span(
                    stmt.else_off,
                    stmt.else_off + "else".len(),
                    "let-else: every path through the `else` block must diverge".to_string(),
                )
                .code(DiagnosticCode::LetElseNotDiverging)
                .owner(stmt.head_span.start, stmt.head_span.end)
                .help(
                    "end it with `return`, `throw`, `break`, or `continue` (an `if`/`else` \
                     counts when both branches do)",
                ),
            );
        }
        self.check_leaf_bindings(&stmt.alternatives[0]);
        self.check_alternatives(&stmt.alternatives, "let-else");
        self.visit_program(&stmt.expr, Ctx::Expr, Place::ValueRegion);
        // The `else` block is inline: its statements run where the
        // statement stands.
        self.visit_program(&stmt.else_body, Ctx::Stmt, place.inline(stmt.in_function));
    }

    /// `if let` emits a self-contained block statement, so it needs a
    /// statement position — which an expression region provides exactly
    /// when the user wrote a function there (the same flow fact that
    /// places `try`, judged from the other side: no value boundary to escape, just
    /// a statement stream to stand in).
    fn check_if_let(&mut self, stmt: &IfLetStmt, ctx: Ctx, place: Place) {
        crate::stack::grow(|| self.check_if_let_grown(stmt, ctx, place));
    }

    fn check_if_let_grown(&mut self, stmt: &IfLetStmt, ctx: Ctx, place: Place) {
        if stmt.expression_position || (ctx == Ctx::Expr && !stmt.in_function) {
            self.error(
                TtError::span(
                    stmt.head_span.start,
                    stmt.head_span.end,
                    "`if let` cannot be used in expression position — it is a statement and \
                     produces no value"
                        .to_string(),
                )
                .code(DiagnosticCode::IfLetPlacement)
                .help(
                    "`match` on the value instead, or write the `if let` as a statement \
                     (inside an expression region, in a function written there)",
                ),
            );
        }
        self.check_leaf_bindings(&stmt.alternatives[0]);
        self.check_alternatives(&stmt.alternatives, "if let");
        self.visit_program(&stmt.expr, Ctx::Expr, Place::ValueRegion);
        // The then/else bodies are inline: their statements run where the
        // statement stands, so a `try` inside them exits the function the
        // chain bottoms out in.
        let inline = place.inline(stmt.in_function);
        self.visit_program(&stmt.body, Ctx::Stmt, inline);
        match &stmt.else_part {
            Some(IfLetElse::Block(block)) => self.visit_program(block, Ctx::Stmt, inline),
            Some(IfLetElse::IfLet(inner)) => self.check_if_let(inner, Ctx::Stmt, inline),
            None => {}
        }
    }

    /// The rules every multi-alternative pattern shares with a match
    /// or-arm ([`Checker::check_match`] keeps its own interleaved copy —
    /// its duplicate-arm bookkeeping decides which alternatives are even
    /// compared): the alternatives share one emitted destructuring, so a
    /// nested pattern cannot ride in them and every alternative must bind
    /// the same (field, name) set. `construct` prefixes the message.
    fn check_alternatives(&mut self, alts: &[TagPattern], construct: &str) {
        if alts.len() < 2 {
            return;
        }
        if let Some(at) = alts.iter().find(|a| has_nested(a)) {
            self.error(
                TtError::span(
                    at.tag_off,
                    at.tag_off + at.tag.len(),
                    format!("{construct}: nested patterns cannot be combined with or-patterns"),
                )
                .code(DiagnosticCode::MatchNestedInOrPattern),
            );
        }
        let shared = !alts.iter().any(has_nested);
        let first_set = binding_set(&alts[0], shared);
        for alt in &alts[1..] {
            if binding_set(alt, shared) != first_set {
                self.error(
                    TtError::span(
                        alt.tag_off,
                        alt.tag_off + alt.tag.len(),
                        format!(
                            "{construct}: or-pattern alternatives must bind the same names — {}",
                            binding_mismatch(&alts[0], alt, shared)
                        ),
                    )
                    .code(DiagnosticCode::MatchOrBindingMismatch),
                );
            }
        }
    }

    /// A `result` block is an expression, so it is allowed anywhere; its
    /// body is the construct's isolated value stream ([`Place::ResultRegion`] — a `try` or
    /// let-else there would return from the *block*, not the enclosing
    /// function), and the bindings and the trailing value are expressions.
    fn check_result_block(&mut self, block: &ResultBlock) {
        let statement_place = if block.value.is_some() {
            Place::ResultValueRegion
        } else {
            Place::ResultRegion
        };
        self.result_blocks.push(block.span.start);
        for item in &block.items {
            let ResultItem::Stmts(stmts) = item;
            self.visit_program(stmts, Ctx::Stmt, statement_place);
        }
        if let Some(value) = &block.value {
            self.visit_program(value, Ctx::Expr, Place::ResultValueRegion);
        }
        self.result_blocks.pop();
        if block.value.is_none() {
            self.check_result_outward_controls(block);
            let completes = self
                .result_completions
                .get(&block.span.start)
                .copied()
                .unwrap_or_else(|| crate::ice::bug!("Result block has no HIR flow fact"));
            if !completes {
                self.error(
                    TtError::span(
                        block.span.start,
                        block.span.end,
                        "`result` can reach the end of its body without a success value"
                            .to_string(),
                    )
                    .code(DiagnosticCode::ResultNoSuccessValue)
                    .help("return a value from every reachable path in this `result` block"),
                );
            }
        }
    }

    fn check_result_outward_controls(&mut self, block: &ResultBlock) {
        let Some(ResultItem::Stmts(body)) = block.items.first() else {
            return;
        };
        for control in crate::flow::outward_controls_in_span(self.source, body, block.body_span) {
            let (span, code, message, help) = match control {
                crate::flow::OutwardControl::Break { span, label: None } => (
                    span,
                    DiagnosticCode::ResultBreakCrossing,
                    "`break` cannot leave a `result` block",
                    "break only a loop or switch written inside this `result` block",
                ),
                crate::flow::OutwardControl::Continue { span, label: None } => (
                    span,
                    DiagnosticCode::ResultContinueCrossing,
                    "`continue` cannot leave a `result` block",
                    "continue only a loop written inside this `result` block",
                ),
                crate::flow::OutwardControl::Yield(span) => (
                    span,
                    DiagnosticCode::ResultYieldCrossing,
                    "`yield` cannot cross a `result` block",
                    "yield outside this `result` block instead",
                ),
                crate::flow::OutwardControl::Break {
                    span,
                    label: Some(_),
                }
                | crate::flow::OutwardControl::Continue {
                    span,
                    label: Some(_),
                } => (
                    span,
                    DiagnosticCode::ResultLabelCrossing,
                    "a labeled control transfer cannot leave a `result` block",
                    "keep the label target inside this `result` block",
                ),
            };
            self.error(
                TtError::span(span.start, span.end, message.to_string())
                    .code(code)
                    .help(help),
            );
        }
    }

    fn check_variant(&mut self, decl: &VariantDecl) {
        let mut seen: Vec<&str> = Vec::new();
        for case in &decl.cases {
            if seen.contains(&case.tag.as_str()) {
                self.error(
                    TtError::span(
                        case.tag_off,
                        case.tag_off + case.tag.len(),
                        format!("variant {}: duplicate case \"{}\"", decl.name, case.tag),
                    )
                    .code(DiagnosticCode::VariantDuplicateCase),
                );
                continue;
            }
            seen.push(&case.tag);
        }

        // A case carries its tag in one fixed property, so a payload field
        // of that name has nowhere to go: the declaration would emit the
        // property twice, and the constructor would overwrite the tag with
        // the payload — the variant would stop being able to say which case
        // it is.
        for case in &decl.cases {
            for field in case.fields.iter().flatten() {
                if field.name != crate::core_ir::VARIANT_TAG_FIELD {
                    continue;
                }
                self.error(
                    TtError::span(
                        field.name_off,
                        field.name_off + field.name.len(),
                        format!(
                            "variant {}: case \"{}\" cannot have a field named `{}` — \
                             that is where the case tag lives",
                            decl.name,
                            case.tag,
                            crate::core_ir::VARIANT_TAG_FIELD
                        ),
                    )
                    .code(DiagnosticCode::VariantFieldShadowsTag)
                    .help("rename the field"),
                );
            }
        }

        for case in &decl.cases {
            let Some(fields) = &case.fields else {
                continue;
            };
            let Some(first_optional) = fields.iter().position(|field| field.optional) else {
                continue;
            };
            for field in fields[first_optional..]
                .iter()
                .filter(|field| !field.optional)
            {
                self.error(
                    TtError::span(
                        field.name_off,
                        field.name_off + field.name.len(),
                        format!(
                            "variant {}: case \"{}\" declares required field `{}` after optional field `{}`",
                            decl.name, case.tag, field.name, fields[first_optional].name
                        ),
                    )
                    .code(DiagnosticCode::VariantRequiredAfterOptional)
                    .help("declare the required fields before the optional ones"),
                );
            }
        }

        if self.verify {
            for case in &decl.cases {
                if let Some(fields) = &case.fields {
                    for field in fields {
                        if let Err(msg) = verify::check_type_fragment(&field.ty) {
                            self.error(
                                TtError::span(
                                    field.ty_off,
                                    field.ty_off + field.ty.len(),
                                    format!(
                                        "variant {}: invalid type for field `{}`: {}",
                                        decl.name, field.name, msg
                                    ),
                                )
                                .code(DiagnosticCode::VariantInvalidFieldType),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Bound names must be unique within one pattern — they all land in the
    /// same scope, so a duplicate would emit two `const`s of one name.
    fn check_leaf_bindings(&mut self, alt: &TagPattern) {
        let mut leaves = Vec::new();
        leaf_bindings(alt, &mut leaves);
        for (i, name) in leaves.iter().enumerate() {
            if leaves[..i].contains(name) {
                self.error(
                    TtError::span(
                        alt.tag_off,
                        alt.tag_off + alt.tag.len(),
                        format!("match: binding `{name}` is used more than once in this pattern"),
                    )
                    .code(DiagnosticCode::PatternDuplicateBinding)
                    .help("rename one of them with `field: alias`"),
                );
            }
        }
    }

    fn check_arm_body(&mut self, pattern: Span, missing: bool) {
        if missing {
            self.error(
                TtError::span(
                    pattern.start,
                    pattern.end,
                    "match: this arm has no body".to_string(),
                )
                .code(DiagnosticCode::MissingArmBody)
                .help("write `=> <body>` after the guard, or the body after `=>`"),
            );
        }
    }

    fn check_match(&mut self, expr: &MatchExpr, place: Place) {
        for arm in &expr.arms {
            self.check_arm_body(arm.pattern_span, arm.missing);
        }
        // Class tests and literals both compare the subject value and may
        // share one ordered conditional chain. Variant tags discriminate on
        // `.kind` and therefore cannot mix with either family.
        let has_tag = expr
            .arms
            .iter()
            .any(|arm| matches!(arm.pattern, Pattern::Tags(_)));
        let has_value_pattern = expr
            .arms
            .iter()
            .any(|arm| matches!(arm.pattern, Pattern::Literals(_) | Pattern::Instances(_)));
        if has_tag
            && has_value_pattern
            && let Some(other) = expr
                .arms
                .iter()
                .find(|arm| matches!(arm.pattern, Pattern::Literals(_) | Pattern::Instances(_)))
        {
            let mixed = if matches!(other.pattern, Pattern::Literals(_)) {
                "match: cannot mix tag patterns and literal patterns in the same match — the two compare different things (`$tt_m.kind` vs `$tt_m`)"
            } else {
                "match: cannot mix tag patterns and `is` patterns in the same match — the two compare different things (`$tt_m.kind` vs `$tt_m`)"
            };
            self.error(
                TtError::span(
                    other.pattern_span.start,
                    other.pattern_span.end,
                    mixed.to_string(),
                )
                .code(DiagnosticCode::MatchMixedPatterns)
                .help("split them into two matches")
                .owner(expr.keyword_off, expr.body_close + 1),
            );
        }

        let has_instances = expr
            .arms
            .iter()
            .any(|arm| matches!(arm.pattern, Pattern::Instances(_)));
        if has_instances {
            // Class hierarchies are open; wildcard presence is the complete
            // exhaustiveness rule, and the coverage question a match with an
            // `is` arm asks is none (`analysis::coverage_question`).
            if !expr
                .arms
                .iter()
                .any(|arm| matches!(arm.pattern, Pattern::Wildcard))
            {
                self.error(
                    TtError::span(
                        expr.keyword_off,
                        expr.keyword_off + "match".len(),
                        "match: an `is` match requires a final wildcard arm `_`".to_string(),
                    )
                    .code(DiagnosticCode::MatchIsWildcardRequired)
                    .help("add `_ => <fallback>` as the last arm")
                    .owner(expr.keyword_off, expr.body_close + 1),
                );
            }
        }

        // Tags covered by an unguarded arm. Any later arm repeating one of
        // these is unreachable (duplicate); a guarded arm covers nothing, so
        // guarded arms may repeat each other's tags.
        let mut covered_tags: Vec<&str> = Vec::new();
        // The same, for literal patterns.
        let mut covered_literals: Vec<&LiteralValue> = Vec::new();
        // Constructor identity is syntax-level and remains covered even when
        // its arm has a guard, as required by the `is` pattern contract.
        let mut covered_instances: Vec<&str> = Vec::new();
        for (idx, arm) in expr.arms.iter().enumerate() {
            match &arm.pattern {
                Pattern::Wildcard => {
                    if idx != expr.arms.len() - 1 {
                        self.error(
                            TtError::span(
                                arm.pattern_span.start,
                                arm.pattern_span.end,
                                "match: the wildcard arm `_` must be the last arm".to_string(),
                            )
                            .code(DiagnosticCode::MatchWildcardNotLast),
                        );
                    }
                }
                Pattern::Literals(alts) => {
                    let mut arm_values: Vec<&LiteralValue> = Vec::new();
                    for alt in alts {
                        if alt.value.kind() != alts[0].value.kind() {
                            self.error(
                                TtError::span(
                                    alt.span.start,
                                    alt.span.end,
                                    format!(
                                        "match: or-pattern alternatives must all be the same kind of \
                                         literal (found {} after {})",
                                        alt.value.kind(),
                                        alts[0].value.kind()
                                    ),
                                )
                                .code(DiagnosticCode::MatchOrLiteralKindMismatch),
                            );
                            continue;
                        }
                        if covered_literals.contains(&&alt.value)
                            || arm_values.contains(&&alt.value)
                        {
                            self.error(
                                TtError::span(
                                    alt.span.start,
                                    alt.span.end,
                                    format!("match: duplicate arm {}", alt.value.render()),
                                )
                                .code(DiagnosticCode::MatchDuplicateArm),
                            );
                            continue;
                        }
                        arm_values.push(&alt.value);
                    }
                    if arm.guard.is_none() {
                        covered_literals.append(&mut arm_values);
                    }
                }
                Pattern::Tags(alts) => {
                    // Codegen emits one destructuring shared by every
                    // alternative (switch fallthrough), so all alternatives
                    // must bind the exact same (field, name) set — which is
                    // also why a nested pattern (per-alternative conditions
                    // and paths) cannot appear inside an or-pattern.
                    if alts.len() > 1
                        && let Some(at) = alts.iter().find(|a| has_nested(a))
                    {
                        self.error(
                            TtError::span(
                                at.tag_off,
                                at.tag_off + at.tag.len(),
                                "match: nested patterns cannot be combined with or-patterns"
                                    .to_string(),
                            )
                            .code(DiagnosticCode::MatchNestedInOrPattern),
                        );
                    }
                    self.check_leaf_bindings(&alts[0]);
                    let shared = !alts.iter().any(has_nested);
                    let first_set = binding_set(&alts[0], shared);
                    let mut arm_tags: Vec<&str> = Vec::new();
                    for alt in alts {
                        if covered_tags.contains(&alt.tag.as_str())
                            || arm_tags.contains(&alt.tag.as_str())
                        {
                            self.error(
                                TtError::span(
                                    alt.tag_off,
                                    alt.tag_off + alt.tag.len(),
                                    format!("match: duplicate arm \"{}\"", alt.tag),
                                )
                                .code(DiagnosticCode::MatchDuplicateArm),
                            );
                            continue;
                        }
                        arm_tags.push(&alt.tag);
                        if binding_set(alt, shared) != first_set {
                            self.error(
                                TtError::span(
                                    alt.tag_off,
                                    alt.tag_off + alt.tag.len(),
                                    format!(
                                        "match: or-pattern alternatives must bind the same names — {}",
                                        binding_mismatch(&alts[0], alt, shared)
                                    ),
                                )
                                .code(DiagnosticCode::MatchOrBindingMismatch),
                            );
                        }
                    }
                    // A nested pattern may mismatch, so — like a guard —
                    // the arm identifies the variant declaration but covers nothing.
                    if arm.guard.is_none() && !alts.iter().any(has_nested) {
                        covered_tags.append(&mut arm_tags);
                    }
                }
                Pattern::Instances(alts) => {
                    if alts.len() > 1
                        && let Some(bound) = alts.iter().find(|alt| alt.bindings.is_some())
                    {
                        self.error(
                            TtError::span(
                                bound.is_off,
                                bound.end,
                                "match: an `is` or-pattern cannot bind properties".to_string(),
                            )
                            .code(DiagnosticCode::MatchIsOrBindings)
                            .help("use type-only alternatives or split them into separate arms"),
                        );
                    }
                    let mut arm_paths: Vec<&str> = Vec::new();
                    for alt in alts {
                        if alt.bindings.as_ref().is_some_and(Vec::is_empty) {
                            self.error(
                                TtError::span(
                                    alt.is_off,
                                    alt.end,
                                    format!(
                                        "match: `is {} {{ }}` has an empty property pattern",
                                        alt.path
                                    ),
                                )
                                .code(DiagnosticCode::MatchIsEmptyBindings)
                                .help("remove the braces"),
                            );
                        }
                        if let Some(bindings) = &alt.bindings {
                            let mut names: Vec<&str> = Vec::new();
                            for binding in bindings {
                                let name = binding.alias.as_deref().unwrap_or(&binding.name);
                                if names.contains(&name) {
                                    self.error(
                                        TtError::span(
                                            binding.name_span.start,
                                            binding.name_span.end,
                                            format!(
                                                "match: binding `{name}` is used more than once in this pattern"
                                            ),
                                        )
                                        .code(DiagnosticCode::PatternDuplicateBinding)
                                        .help("rename one of them with `field: alias`"),
                                    );
                                } else {
                                    names.push(name);
                                }
                            }
                        }
                        if covered_instances.contains(&alt.path.as_str())
                            || arm_paths.contains(&alt.path.as_str())
                        {
                            self.error(
                                TtError::span(
                                    alt.path_span.start,
                                    alt.path_span.end,
                                    format!("match: duplicate arm `is {}`", alt.path),
                                )
                                .code(DiagnosticCode::MatchDuplicateArm),
                            );
                        } else {
                            arm_paths.push(&alt.path);
                        }
                    }
                    // Constructor identity is structural and independent of
                    // guards: the RFC deliberately rejects two arms naming
                    // the same path even when their guards differ.
                    covered_instances.extend(arm_paths);
                }
            }
        }

        // Exhaustiveness is not recorded here: the analysis walks the same
        // program and answers for every match at once (`report_coverage`).

        // children, in source order: scrutinee first, then guards and bodies
        let isolated = place.isolated();
        self.visit_program(&expr.scrutinee, Ctx::Expr, place);
        for arm in &expr.arms {
            if let Some(guard) = &arm.guard {
                self.visit_program(&guard.expr, Ctx::Expr, isolated);
            }
            if arm.block {
                self.check_match_arm_controls(&arm.body, arm.body_span);
            }
            // A block arm body is a statement context inside the value region.
            self.visit_program(
                &arm.body,
                if arm.block { Ctx::Stmt } else { Ctx::Expr },
                isolated,
            );
        }
    }

    fn check_match_arm_controls(&mut self, body: &Program, body_span: Span) {
        for control in crate::flow::outward_controls_in_span(self.source, body, body_span) {
            let (span, message, help) = match control {
                crate::flow::OutwardControl::Break { span, .. } => (
                    span,
                    "`break` cannot leave a match arm",
                    "break only a loop or switch written inside this arm",
                ),
                crate::flow::OutwardControl::Continue { span, .. } => (
                    span,
                    "`continue` cannot leave a match arm",
                    "continue only a loop written inside this arm",
                ),
                crate::flow::OutwardControl::Yield(span) => (
                    span,
                    "`yield` cannot cross a match arm boundary",
                    "yield outside the match, or yield inside a generator written in this arm",
                ),
            };
            self.error(
                TtError::span(span.start, span.end, message.to_string())
                    .code(DiagnosticCode::MatchControlCrossing)
                    .help(help),
            );
        }
    }

    fn check_tuple_match(&mut self, expr: &TupleMatchExpr, place: Place) {
        for arm in &expr.arms {
            self.check_arm_body(arm.pattern_span, arm.missing);
        }
        let arity = expr.scrutinees.len();
        for (idx, arm) in expr.arms.iter().enumerate() {
            match &arm.pattern {
                TuplePattern::Wildcard => {
                    if idx != expr.arms.len() - 1 {
                        self.error(
                            TtError::span(
                                arm.pattern_span.start,
                                arm.pattern_span.end,
                                "match: the wildcard arm `_` must be the last arm".to_string(),
                            )
                            .code(DiagnosticCode::MatchWildcardNotLast),
                        );
                    }
                }
                TuplePattern::Elems(elems) => {
                    if elems.len() != arity {
                        let elements = if elems.len() == 1 {
                            "element"
                        } else {
                            "elements"
                        };
                        let scrutinees = if arity == 1 {
                            "scrutinee"
                        } else {
                            "scrutinees"
                        };
                        let owner = expr.head_span();
                        self.error(
                            TtError::span(
                                arm.pattern_span.start,
                                arm.pattern_span.end,
                                format!(
                                    "match: tuple pattern has {} {elements} but the match has {} {scrutinees}",
                                    elems.len(), arity
                                ),
                            )
                            .code(DiagnosticCode::MatchTupleArity)
                            .owner(owner.start, owner.end),
                        );
                    }
                    // Every element's or-alternatives share one
                    // destructuring (hence no nested patterns in them);
                    // bound names must also be unique across the whole
                    // tuple pattern (they land in one scope).
                    let mut bound: Vec<&str> = Vec::new();
                    for elem in elems {
                        let Pattern::Tags(alts) = elem else { continue };
                        if alts.len() > 1
                            && let Some(at) = alts.iter().find(|a| has_nested(a))
                        {
                            self.error(
                                TtError::span(
                                    at.tag_off,
                                    at.tag_off + at.tag.len(),
                                    "match: nested patterns cannot be combined with or-patterns"
                                        .to_string(),
                                )
                                .code(DiagnosticCode::MatchNestedInOrPattern),
                            );
                        }
                        let shared = !alts.iter().any(has_nested);
                        let first_set = binding_set(&alts[0], shared);
                        for alt in alts {
                            if binding_set(alt, shared) != first_set {
                                self.error(
                                    TtError::span(
                                        alt.tag_off,
                                        alt.tag_off + alt.tag.len(),
                                        format!(
                                            "match: or-pattern alternatives must bind the same names — {}",
                                            binding_mismatch(&alts[0], alt, shared)
                                        ),
                                    )
                                    .code(DiagnosticCode::MatchOrBindingMismatch),
                                );
                            }
                        }
                        let mut leaves = Vec::new();
                        leaf_bindings(&alts[0], &mut leaves);
                        for name in leaves {
                            if bound.contains(&name) {
                                self.error(
                                    TtError::span(
                                        alts[0].tag_off,
                                        alts[0].tag_off + alts[0].tag.len(),
                                        format!(
                                            "match: binding `{name}` is used more than once in this tuple pattern"
                                        ),
                                    )
                                    .code(DiagnosticCode::PatternDuplicateBinding)
                                    .help("rename one of them with `field: alias`"),
                                );
                                continue;
                            }
                            bound.push(name);
                        }
                    }
                }
            }
        }

        // children, in source order
        let isolated = place.isolated();
        for (_, scrutinee) in &expr.scrutinees {
            self.visit_program(scrutinee, Ctx::Expr, place);
        }
        for arm in &expr.arms {
            if let Some(guard) = &arm.guard {
                self.visit_program(&guard.expr, Ctx::Expr, isolated);
            }
            if arm.block {
                self.check_match_arm_controls(&arm.body, arm.body_span);
            }
            self.visit_program(
                &arm.body,
                if arm.block { Ctx::Stmt } else { Ctx::Expr },
                isolated,
            );
        }
    }
}
