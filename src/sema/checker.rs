//! Semantic traversal and construct-specific validation.

use super::*;

impl Checker {
    fn error(&mut self, error: TtError) {
        self.errors.push(error);
    }

    pub(super) fn visit_program(&mut self, program: &Program, ctx: Ctx, place: Place) {
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
        for &off in &program.stray_if_lets {
            self.error(
                TtError::span(
                    off,
                    off + "if".len(),
                    "`if let` could not be parsed here".to_string(),
                )
                .code(DiagnosticCode::StrayIfLet)
                .help(
                    "the pattern parens are mandatory, and the `else` must be a block or \
                     another `if let`",
                ),
            );
        }
        for &off in &program.stray_results {
            self.error(
                TtError::span(
                    off,
                    off + "result".len(),
                    "`result` block could not be parsed here".to_string(),
                )
                .code(DiagnosticCode::StrayResult)
                .help(
                    "use `const binding = try expression;` and finish every reachable success \
                     path with `return`",
                ),
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
                        self.visit_program(head, Ctx::Expr, place.isolated());
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

    /// `try` placement is a **flow** fact, not a nesting rule: the lowering
    /// emits a `return`, so the statement must run inside a user-written
    /// function — one written in its own region (a `try` inside an arrow
    /// in a match arm, a scrutinee, a pipeline step is fine, exactly like
    /// `?` inside a closure in Rust), or one an inline chain (an `if let`
    /// body, a let-else `else` block) bottoms out in. Without one, the
    /// `return` would exit the construct's own value region, or fall at the
    /// module's top level, where there is nothing to return from.
    fn check_try(&mut self, stmt: &TryStmt, place: Place) {
        let at = self
            .tokens
            .iter()
            .position(|token| token.span.start >= stmt.span.start)
            .unwrap_or(self.tokens.len());
        let function_target = crate::flow::function_target_at(&self.source, &self.tokens, at);
        if place != Place::ResultRegion
            && matches!(
                function_target,
                Some(
                    crate::flow::FunctionTarget::Constructor
                        | crate::flow::FunctionTarget::Generator
                )
            )
        {
            self.error(
                TtError::span(
                    stmt.span.start,
                    stmt.span.end,
                    "`try` cannot be used in a constructor or generator — its `Err` propagation requires an ordinary function return".to_string(),
                )
                .code(DiagnosticCode::TryPlacement)
                .help("move the propagation into an ordinary function, or handle the Result explicitly"),
            );
        }
        if place == Place::ResultValueRegion {
            self.error(
                TtError::span(
                    stmt.span.start,
                    stmt.span.end,
                    "`try` crosses an isolated value region whose exits cannot target the enclosing `result` block".to_string(),
                )
                .code(DiagnosticCode::TryCrossesValueRegion)
                .help("extract the affected expression into a nested function when doing so preserves its captures and evaluation order"),
            );
        } else if place != Place::ResultRegion
            && function_target.is_none()
            && crate::flow::in_static_block(&self.source, &self.tokens, at)
        {
            self.error(
                TtError::span(
                    stmt.span.start,
                    stmt.span.end,
                    "`try` cannot be used in a class static block — it has no enclosing function failure edge for its `Err` propagation".to_string(),
                )
                .code(DiagnosticCode::TryPlacement)
                .help("move the propagation into an ordinary function, or handle the Result explicitly"),
            );
        } else {
            self.check_try_placement(stmt.span, stmt.in_function, place);
        }
        self.visit_program(&stmt.expr, Ctx::Expr, Place::ValueRegion);
    }

    fn check_try_expr(&mut self, expr: &crate::ast::TryExpr, place: Place) {
        // Ordinary expression propagation is judged after the SWC host owner
        // and evaluation protocol are known. A result block is the one
        // surface-owned boundary: its direct propagation targets that region.
        if place == Place::ResultValueRegion {
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

    fn check_try_placement(&mut self, span: Span, in_function: bool, place: Place) {
        if place == Place::ResultRegion {
            return;
        }
        if !in_function && place != Place::Function {
            let (message, help) = if place == Place::Module {
                (
                    "`try` must be inside a function — it compiles to a `return` that \
                     propagates the `Err`, and at the top level of a module there is no \
                     function to return from",
                    "move the code into a function whose `Err` this can return, or `match` \
                     on the `Result` instead",
                )
            } else {
                (
                    "`try` cannot be used here, in an isolated value region — it compiles to \
                     a `return`, which would exit this construct's own IIFE instead of the \
                     enclosing function",
                    "extract the logic into a function (a `try` inside a function written \
                     here is fine), or move the propagation into a statement-bodied `result` block",
                )
            };
            self.error(
                TtError::span(span.start, span.end, message.to_string())
                    .code(DiagnosticCode::TryPlacement)
                    .help(help),
            );
        }
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
                     `break`, `continue`) would leave this construct's own IIFE instead of \
                     the enclosing function"
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
        if ctx == Ctx::Expr && !stmt.in_function {
            self.error(
                TtError::span(
                    stmt.head_span.start,
                    stmt.head_span.end,
                    "`if let` cannot be used in expression position (a template \
                     interpolation, a scrutinee or guard, an expression arm body, a `try` \
                     expression, or a pipeline) — it compiles to a block statement"
                        .to_string(),
                )
                .code(DiagnosticCode::IfLetPlacement)
                .help(
                    "write it inside a function here (an `if let` in one is fine), or \
                     `match` on the value instead",
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
        let first_set = binding_set(&alts[0].bindings);
        for alt in &alts[1..] {
            if binding_set(&alt.bindings) != first_set {
                self.error(
                    TtError::span(
                        alt.tag_off,
                        alt.tag_off + alt.tag.len(),
                        format!(
                            "{construct}: or-pattern alternatives must bind the same names — {}",
                            binding_mismatch(&alts[0], alt)
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
        for item in &block.items {
            let ResultItem::Stmts(stmts) = item;
            self.visit_program(stmts, Ctx::Stmt, statement_place);
        }
        if let Some(value) = &block.value {
            self.visit_program(value, Ctx::Expr, Place::ResultValueRegion);
        } else {
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
        for control in
            crate::flow::outward_controls_in_span(&self.source, &self.tokens, body, block.body_span)
        {
            let (span, code, message, help) = match control {
                crate::flow::OutwardControl::Break {
                    span,
                    labeled: false,
                } => (
                    span,
                    DiagnosticCode::ResultBreakCrossing,
                    "`break` cannot leave a `result` block",
                    "break only a loop or switch written inside this `result` block",
                ),
                crate::flow::OutwardControl::Continue {
                    span,
                    labeled: false,
                } => (
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
                    labeled: true,
                }
                | crate::flow::OutwardControl::Continue {
                    span,
                    labeled: true,
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

    fn check_match(&mut self, expr: &MatchExpr, place: Place) {
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
            // A mixed match has no one discriminant, so its coverage answer
            // is not worth asking — report the cause, not its effects.
            self.coverage_suppressed.push(expr.keyword_off);
        }

        let has_instances = expr
            .arms
            .iter()
            .any(|arm| matches!(arm.pattern, Pattern::Instances(_)));
        if has_instances {
            // Class hierarchies are open; wildcard presence is the complete
            // exhaustiveness rule and the variant/literal coverage engines
            // must not infer anything else for this site.
            self.coverage_suppressed.push(expr.keyword_off);
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
                    let first_set = binding_set(&alts[0].bindings);
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
                        if binding_set(&alt.bindings) != first_set {
                            self.error(
                                TtError::span(
                                    alt.tag_off,
                                    alt.tag_off + alt.tag.len(),
                                    format!(
                                        "match: or-pattern alternatives must bind the same names — {}",
                                        binding_mismatch(&alts[0], alt)
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
        self.visit_program(&expr.scrutinee, Ctx::Expr, isolated);
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
        for control in
            crate::flow::outward_controls_in_span(&self.source, &self.tokens, body, body_span)
        {
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
                        let first_set = binding_set(&alts[0].bindings);
                        for alt in alts {
                            if binding_set(&alt.bindings) != first_set {
                                self.error(
                                    TtError::span(
                                        alt.tag_off,
                                        alt.tag_off + alt.tag.len(),
                                        format!(
                                            "match: or-pattern alternatives must bind the same names — {}",
                                            binding_mismatch(&alts[0], alt)
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
            self.visit_program(scrutinee, Ctx::Expr, isolated);
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
