//! TypeScript projection construction and the public syntax view.

mod segments;

pub(super) use segments::{ProjectionSegmentKind, ProjectionSegments, ProjectionSourceSegment};

use super::*;

/// A complete SWC view of one tt-containing TypeScript module.
#[derive(Debug)]
pub(crate) struct ProgramSyntax {
    pub(super) source_len: usize,
    pub(super) projection: String,
    pub(super) module: Module,
    pub(super) overlay: Vec<OverlayEntry>,
    pub(super) owners: Vec<HostOwnerSyntax>,
    pub(super) occupied_names: HashSet<String>,
    pub(super) directive_prologue_end: Option<usize>,
    pub(super) script: bool,
    pub(super) commonjs: bool,
    pub(super) globals: HashMap<SourceSpan, GlobalStatement>,
    pub(super) completion_scopes: Vec<super::completion::CompletionScope>,
}

#[derive(Debug)]
pub(crate) struct HostOwnerSyntax {
    pub(crate) owner: HostOwner,
    pub(crate) roots: Vec<TtNodeId>,
}

/// Why a shadow program model could not be built.
///
/// Every variant but [`ProgramSyntaxError::SourceNotTypeScript`] is a broken
/// compiler invariant — a validator failure in the sense of
/// `docs/design/program-lowering.md` §11, and therefore an internal compiler
/// error. The projection's *parse*, though, is not a validator: the
/// projection is TypeScript the user wrote with tt values replaced by
/// placeholders, so it can also fail because that TypeScript is not
/// TypeScript. That cause is a fact about the input and carries the source
/// byte to report it at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProgramSyntaxError {
    MissingSourceSpan {
        node: NodeId,
    },
    InvalidSourceSpan {
        start: SourceByte,
        end: SourceByte,
    },
    NodeCountOverflow,
    /// The projection stopped parsing at a byte copied verbatim from the
    /// source: the user's own TypeScript does not parse.
    SourceNotTypeScript {
        message: String,
        source: usize,
    },
    /// The projection stopped parsing at a byte this compiler generated.
    /// `source` is the construct whose placeholder holds that byte: the
    /// innermost one, since a construct's placeholder can enclose those of
    /// the constructs nested in it.
    Parse {
        message: String,
        projection: String,
        source: Option<SourceSpan>,
    },
    MissingOverlay {
        id: TtNodeId,
    },
    DuplicateOverlay {
        id: TtNodeId,
    },
    UnmappedEvaluationSpan {
        start: usize,
        end: usize,
    },
}

impl std::fmt::Display for ProgramSyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProgramSyntaxError::MissingSourceSpan { .. } => {
                write!(f, "a tt node has no source span")
            }
            ProgramSyntaxError::InvalidSourceSpan { start, end } => {
                write!(
                    f,
                    "a tt node's source span {}..{} is invalid",
                    start.0, end.0
                )
            }
            ProgramSyntaxError::NodeCountOverflow => write!(f, "too many tt nodes in one file"),
            ProgramSyntaxError::SourceNotTypeScript { message, .. } => {
                write!(f, "the TypeScript here does not parse: {message}")
            }
            ProgramSyntaxError::Parse { message, .. } => {
                write!(
                    f,
                    "the generated TypeScript for it does not parse: {message}"
                )
            }
            ProgramSyntaxError::MissingOverlay { .. } => {
                write!(
                    f,
                    "the construct has no place in the TypeScript syntax tree"
                )
            }
            ProgramSyntaxError::DuplicateOverlay { .. } => {
                write!(
                    f,
                    "the construct has two places in the TypeScript syntax tree"
                )
            }
            ProgramSyntaxError::UnmappedEvaluationSpan { start, end } => {
                write!(
                    f,
                    "the evaluation position {start}..{end} maps to no source"
                )
            }
        }
    }
}

impl ProgramSyntax {
    /// Builds and validates the shadow program model.
    #[cfg(test)]
    pub(crate) fn build(
        semantic: &SemanticFile,
        core: &CoreFile,
        source: &str,
        source_kind: crate::SourceKind,
    ) -> Result<Self, ProgramSyntaxError> {
        let tokens = crate::lexer::lex_with_kind(source, 0, source.len(), source_kind);
        Self::build_with(
            semantic,
            core,
            source,
            source_kind,
            &tokens,
            SyntaxMode::Strict,
        )
    }

    pub(crate) fn build_with(
        semantic: &SemanticFile,
        core: &CoreFile,
        source: &str,
        source_kind: crate::SourceKind,
        tokens: &[crate::lexer::Token],
        mode: SyntaxMode,
    ) -> Result<Self, ProgramSyntaxError> {
        let error = if mode == SyntaxMode::Editor {
            crate::lexer::host_lexical_error_in(source, source_kind, tokens)
        } else {
            crate::lexer::host_syntax_error_in(source, source_kind, tokens)
        };
        if let Some((span, message)) = error {
            return Err(ProgramSyntaxError::SourceNotTypeScript {
                message: message.to_string(),
                source: span.start,
            });
        }
        let projection = ProjectionBuilder::new(semantic, core, source, tokens).build()?;
        let parsed = parse_module(
            &projection.code,
            &projection.source_segments,
            source_kind,
            mode,
        )?;
        for part in &projection.hidden_parts {
            let text = &source[part.source.start..part.source.end];
            let (receiver, member) = match part.role {
                HiddenRole::Step { postfix: true } => ("$tt_syntax_piped", ""),
                HiddenRole::Head { receiver: true }
                    if crate::lexer::is_member_receiver(text, 0, text.len(), source_kind) =>
                {
                    ("", ".$tt_syntax")
                }
                HiddenRole::Head { .. } | HiddenRole::Step { .. } => ("", ""),
            };
            let open = "class $tt_syntax extends Object { async *$tt_syntax() { (";
            let start = open.len() + receiver.len();
            let code = format!("{open}{receiver}{text}{member}); }} }}");
            let segments = [ProjectionSourceSegment {
                projected: ProjectedSpan {
                    start: ProjectedByte(start),
                    end: ProjectedByte(start + text.len()),
                },
                source: part.source,
                kind: ProjectionSegmentKind::Copied,
            }];
            parse_module(&code, &segments, source_kind, mode).map_err(|error| match error {
                ProgramSyntaxError::Parse { message, .. } => {
                    ProgramSyntaxError::SourceNotTypeScript {
                        message,
                        source: part.source.start,
                    }
                }
                error => error,
            })?;
        }
        let completion_scopes = super::completion::completion_scopes(
            &parsed.module,
            parsed.start,
            &projection.pending,
            &projection.source_segments,
            &projection.completion,
        );
        let mut collector = ParentCollector::new(
            parsed.start,
            &projection.pending,
            &projection.source_segments,
            &projection.projection_only_protocol_parents,
            &projection.arm_blocks,
            &projection.tt_bindings,
        );
        let script = is_script(&parsed.module);
        let commonjs = uses_commonjs_syntax(&parsed.module);
        if script {
            collector.global_statements = global_statements(&parsed.module, parsed.start);
        }
        let mut path = AstNodePath::default();
        parsed.module.visit_with_ast_path(&mut collector, &mut path);
        let mut collected = collector.finish(projection.pending)?;
        collected
            .occupied_names
            .extend(crate::generated_names::source_names(source, source_kind));
        let directive_prologue_end =
            directive_prologue_end(&parsed.module, parsed.start, &projection.source_segments)?;
        let mut globals = collected.globals;
        for entry in &collected.overlay {
            let CoreRoot::Decision(extent) = entry.core_root else {
                continue;
            };
            let Some(global) = globals.get_mut(&entry.host_owner.statement()) else {
                continue;
            };
            if let Some(binding) = let_else_global_binding(semantic, core, source, extent) {
                *global = binding;
            }
        }
        let syntax = Self {
            directive_prologue_end,
            source_len: source.len(),
            projection: projection.code,
            module: parsed.module,
            overlay: collected.overlay,
            owners: collected.owners,
            occupied_names: collected.occupied_names,
            script,
            commonjs,
            globals,
            completion_scopes,
        };
        syntax.validate()?;
        Ok(syntax)
    }

    /// Returns the Core roots joined to their TypeScript host contexts.
    pub(crate) fn core_contexts(
        &self,
    ) -> impl Iterator<
        Item = (
            CoreRoot,
            TtNodeId,
            EvaluationContext,
            HostEvaluationProtocol,
            SourceSpan,
            HostOwner,
            Vec<HostExit>,
        ),
    > + '_ {
        self.overlay.iter().map(|entry| {
            (
                entry.core_root,
                entry.id,
                entry.context,
                entry.protocol.clone(),
                entry.source,
                entry.host_owner,
                entry.exits.clone(),
            )
        })
    }

    pub(crate) fn owners(&self) -> impl Iterator<Item = &HostOwnerSyntax> {
        self.owners.iter()
    }

    /// What TypeScript's completion rules say at each construct's place.
    pub(crate) fn take_completion_scopes(&mut self) -> Vec<super::completion::CompletionScope> {
        std::mem::take(&mut self.completion_scopes)
    }

    pub(crate) fn directive_prologue_end(&self) -> Option<usize> {
        self.directive_prologue_end
    }

    pub(crate) fn is_script(&self) -> bool {
        self.script
    }

    pub(crate) fn uses_commonjs_syntax(&self) -> bool {
        self.commonjs
    }

    pub(crate) fn globals(&self) -> &HashMap<SourceSpan, GlobalStatement> {
        &self.globals
    }

    pub(crate) fn occupied_names(&self) -> impl Iterator<Item = &str> {
        self.occupied_names.iter().map(String::as_str)
    }

    pub(crate) fn declared_names(&self) -> HashSet<String> {
        use swc_ecma_visit::{Visit, VisitWith};

        struct Declarations(HashSet<String>);
        impl Visit for Declarations {
            fn visit_binding_ident(&mut self, node: &swc_ecma_ast::BindingIdent) {
                self.0.insert(node.id.sym.to_string());
            }
            fn visit_fn_decl(&mut self, node: &swc_ecma_ast::FnDecl) {
                self.0.insert(node.ident.sym.to_string());
                node.visit_children_with(self);
            }
            fn visit_fn_expr(&mut self, node: &swc_ecma_ast::FnExpr) {
                if let Some(ident) = &node.ident {
                    self.0.insert(ident.sym.to_string());
                }
                node.visit_children_with(self);
            }
            fn visit_class_decl(&mut self, node: &swc_ecma_ast::ClassDecl) {
                self.0.insert(node.ident.sym.to_string());
                node.visit_children_with(self);
            }
            fn visit_class_expr(&mut self, node: &swc_ecma_ast::ClassExpr) {
                if let Some(ident) = &node.ident {
                    self.0.insert(ident.sym.to_string());
                }
                node.visit_children_with(self);
            }
            fn visit_import_named_specifier(&mut self, node: &swc_ecma_ast::ImportNamedSpecifier) {
                self.0.insert(node.local.sym.to_string());
            }
            fn visit_import_default_specifier(
                &mut self,
                node: &swc_ecma_ast::ImportDefaultSpecifier,
            ) {
                self.0.insert(node.local.sym.to_string());
            }
            fn visit_import_star_as_specifier(
                &mut self,
                node: &swc_ecma_ast::ImportStarAsSpecifier,
            ) {
                self.0.insert(node.local.sym.to_string());
            }
            fn visit_ts_enum_decl(&mut self, node: &swc_ecma_ast::TsEnumDecl) {
                self.0.insert(node.id.sym.to_string());
                node.visit_children_with(self);
            }
            fn visit_ts_module_decl(&mut self, node: &swc_ecma_ast::TsModuleDecl) {
                if let swc_ecma_ast::TsModuleName::Ident(ident) = &node.id {
                    self.0.insert(ident.sym.to_string());
                }
                node.visit_children_with(self);
            }
            fn visit_ts_import_equals_decl(&mut self, node: &swc_ecma_ast::TsImportEqualsDecl) {
                self.0.insert(node.id.sym.to_string());
            }
        }
        let mut declarations = Declarations(HashSet::new());
        self.module.visit_with(&mut declarations);
        declarations.0
    }

    pub(crate) fn module_declared_names(&self) -> HashSet<String> {
        use swc_ecma_ast::{
            ArrowExpr, Class, Decl, DefaultDecl, GetterProp, ModuleDecl, ObjectPatProp, SetterProp,
            TsModuleDecl, VarDecl, VarDeclKind,
        };
        use swc_ecma_visit::{Visit, VisitWith};

        fn pattern_names(pattern: &Pat, names: &mut HashSet<String>) {
            match pattern {
                Pat::Ident(binding) => {
                    names.insert(binding.id.sym.to_string());
                }
                Pat::Array(array) => {
                    for element in array.elems.iter().flatten() {
                        pattern_names(element, names);
                    }
                }
                Pat::Rest(rest) => pattern_names(&rest.arg, names),
                Pat::Object(object) => {
                    for property in &object.props {
                        match property {
                            ObjectPatProp::KeyValue(pair) => pattern_names(&pair.value, names),
                            ObjectPatProp::Assign(assign) => {
                                names.insert(assign.key.id.sym.to_string());
                            }
                            ObjectPatProp::Rest(rest) => pattern_names(&rest.arg, names),
                        }
                    }
                }
                Pat::Assign(assign) => pattern_names(&assign.left, names),
                Pat::Expr(_) | Pat::Invalid(_) => {}
            }
        }

        fn declaration_names(declaration: &Decl, names: &mut HashSet<String>) {
            match declaration {
                Decl::Class(class) => {
                    names.insert(class.ident.sym.to_string());
                }
                Decl::Fn(function) => {
                    names.insert(function.ident.sym.to_string());
                }
                Decl::Var(var) => {
                    for declarator in &var.decls {
                        pattern_names(&declarator.name, names);
                    }
                }
                Decl::Using(using) => {
                    for declarator in &using.decls {
                        pattern_names(&declarator.name, names);
                    }
                }
                Decl::TsEnum(declaration) => {
                    names.insert(declaration.id.sym.to_string());
                }
                Decl::TsModule(declaration) => {
                    if let swc_ecma_ast::TsModuleName::Ident(ident) = &declaration.id {
                        names.insert(ident.sym.to_string());
                    }
                }
                Decl::TsInterface(_) | Decl::TsTypeAlias(_) => {}
            }
        }

        struct HoistedVars(HashSet<String>);
        impl Visit for HoistedVars {
            fn visit_var_decl(&mut self, node: &VarDecl) {
                if node.kind == VarDeclKind::Var {
                    for declarator in &node.decls {
                        pattern_names(&declarator.name, &mut self.0);
                    }
                }
                node.visit_children_with(self);
            }
            fn visit_function(&mut self, _: &Function) {}
            fn visit_arrow_expr(&mut self, _: &ArrowExpr) {}
            fn visit_class(&mut self, _: &Class) {}
            fn visit_getter_prop(&mut self, _: &GetterProp) {}
            fn visit_setter_prop(&mut self, _: &SetterProp) {}
            fn visit_ts_module_decl(&mut self, _: &TsModuleDecl) {}
        }

        let mut names = HashSet::new();
        for item in &self.module.body {
            match item {
                ModuleItem::Stmt(Stmt::Decl(declaration)) => {
                    declaration_names(declaration, &mut names);
                }
                ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) => {
                    declaration_names(&export.decl, &mut names);
                }
                ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(export)) => {
                    let ident = match &export.decl {
                        DefaultDecl::Class(class) => class.ident.as_ref(),
                        DefaultDecl::Fn(function) => function.ident.as_ref(),
                        DefaultDecl::TsInterfaceDecl(_) => None,
                    };
                    if let Some(ident) = ident {
                        names.insert(ident.sym.to_string());
                    }
                }
                ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
                    for specifier in &import.specifiers {
                        names.insert(specifier.local().sym.to_string());
                    }
                }
                ModuleItem::ModuleDecl(ModuleDecl::TsImportEquals(import)) => {
                    names.insert(import.id.sym.to_string());
                }
                _ => {}
            }
        }
        let mut hoisted = HoistedVars(names);
        self.module.visit_with(&mut hoisted);
        hoisted.0
    }

    fn validate(&self) -> Result<(), ProgramSyntaxError> {
        let _module_span = self.module.span;
        let projection_len = self.projection.len();
        for entry in &self.overlay {
            let start = entry.projected.start.0;
            let end = entry.projected.end.0;
            if start >= end || end > projection_len {
                return Err(ProgramSyntaxError::InvalidSourceSpan {
                    start: SourceByte(entry.source.start),
                    end: SourceByte(entry.source.end),
                });
            }
            if entry.parents.is_empty() {
                return Err(ProgramSyntaxError::MissingOverlay { id: entry.id });
            }
            match entry.category {
                SyntaxCategory::Expression
                | SyntaxCategory::Propagation
                | SyntaxCategory::Statement
                | SyntaxCategory::Item => {}
            }
            match entry.context.continuation {
                HostContinuation::Return
                | HostContinuation::ArrowReturn
                | HostContinuation::Initialize
                | HostContinuation::ForInitialize
                | HostContinuation::Discard
                | HostContinuation::Compose => {}
            }
            for step in entry.protocol.steps() {
                if step.parent.start >= step.parent.end || step.parent.end > self.source_len {
                    return Err(ProgramSyntaxError::InvalidSourceSpan {
                        start: SourceByte(step.parent.start),
                        end: SourceByte(step.parent.end),
                    });
                }
            }
        }
        for (index, owner) in self.owners.iter().enumerate() {
            if owner.owner.id.0 as usize != index || owner.roots.is_empty() {
                return Err(ProgramSyntaxError::NodeCountOverflow);
            }
            if owner.owner.span.start >= owner.owner.span.end
                || owner.owner.span.end > self.source_len
            {
                return Err(ProgramSyntaxError::InvalidSourceSpan {
                    start: SourceByte(owner.owner.span.start),
                    end: SourceByte(owner.owner.span.end),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct TtBindings {
    pub(super) scopes: Vec<(ProjectedSpan, Vec<String>)>,
    pub(super) statements: Vec<(ProjectedSpan, Vec<String>)>,
}

pub(super) struct Projection {
    pub(super) completion: super::completion::CompletionMarks,
    pub(super) arm_blocks: HashMap<ProjectedSpan, BodyId>,
    pub(super) tt_bindings: TtBindings,
    pub(super) code: String,
    pub(super) pending: Vec<PendingOverlay>,
    pub(super) source_segments: Vec<ProjectionSourceSegment>,
    pub(super) projection_only_protocol_parents: Vec<ProjectedSpan>,
    pub(super) hidden_parts: Vec<HiddenPart>,
}

#[derive(Debug)]
pub(super) struct PendingOverlay {
    pub(super) id: TtNodeId,
    pub(super) category: SyntaxCategory,
    pub(super) source: SourceSpan,
    pub(super) projected: ProjectedSpan,
    pub(super) core_root: CoreRoot,
    pub(super) marker: OverlayMarker,
    pub(super) synthetic_return: Option<ProjectedSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OverlayMarker {
    Identifier,
    CallExpression,
    DecisionCallExpression,
}

pub(super) struct ProjectionBuilder<'a> {
    pub(super) completion: super::completion::CompletionMarks,
    pub(super) arm_blocks: HashMap<ProjectedSpan, BodyId>,
    pub(super) tt_bindings: TtBindings,
    pub(super) semantic: &'a SemanticFile,
    pub(super) core: &'a CoreFile,
    pub(super) source: &'a str,
    pub(super) code: String,
    pub(super) pending: Vec<PendingOverlay>,
    pub(super) source_segments: Vec<ProjectionSourceSegment>,
    pub(super) projection_only_protocol_parents: Vec<ProjectedSpan>,
    pub(super) automatic_semicolons: Vec<crate::lexer::AutomaticSemicolon>,
    pub(super) hidden_parts: Vec<HiddenPart>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct HiddenPart {
    pub(super) source: SourceSpan,
    pub(super) role: HiddenRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HiddenRole {
    Head { receiver: bool },
    Step { postfix: bool },
}

impl<'a> ProjectionBuilder<'a> {
    pub(super) fn new(
        semantic: &'a SemanticFile,
        core: &'a CoreFile,
        source: &'a str,
        tokens: &[crate::lexer::Token],
    ) -> Self {
        Self {
            completion: super::completion::CompletionMarks::default(),
            arm_blocks: HashMap::new(),
            tt_bindings: TtBindings::default(),
            semantic,
            core,
            source,
            code: String::with_capacity(source.len()),
            pending: Vec::new(),
            source_segments: Vec::new(),
            projection_only_protocol_parents: Vec::new(),
            automatic_semicolons: crate::lexer::automatic_semicolons(tokens),
            hidden_parts: Vec::new(),
        }
    }

    pub(super) fn build(mut self) -> Result<Projection, ProgramSyntaxError> {
        self.emit_body(self.core.root)?;
        Ok(Projection {
            completion: self.completion,
            arm_blocks: self.arm_blocks,
            tt_bindings: self.tt_bindings,
            code: self.code,
            pending: self.pending,
            source_segments: self.source_segments,
            projection_only_protocol_parents: self.projection_only_protocol_parents,
            hidden_parts: self.hidden_parts,
        })
    }

    fn source_span(&self, node: NodeId) -> Result<SourceSpan, ProgramSyntaxError> {
        self.semantic
            .hir
            .source_map
            .node_span(node)
            .map(SourceSpan::from)
            .ok_or(ProgramSyntaxError::MissingSourceSpan { node })
    }

    fn push_source(&mut self, node: NodeId) -> Result<(), ProgramSyntaxError> {
        let span = self.source_span(node)?;
        let text =
            self.source
                .get(span.start..span.end)
                .ok_or(ProgramSyntaxError::InvalidSourceSpan {
                    start: SourceByte(span.start),
                    end: SourceByte(span.end),
                })?;
        let start = ProjectedByte(self.code.len());
        self.code.push_str(text);
        let end = ProjectedByte(self.code.len());
        self.source_segments.push(ProjectionSourceSegment {
            projected: ProjectedSpan { start, end },
            source: span,
            kind: ProjectionSegmentKind::Copied,
        });
        Ok(())
    }

    fn push_placeholder(
        &mut self,
        category: SyntaxCategory,
        source: SourceSpan,
        core_root: CoreRoot,
    ) -> Result<(), ProgramSyntaxError> {
        let owner_start = ProjectedByte(self.code.len());
        match category {
            SyntaxCategory::Expression | SyntaxCategory::Propagation => self.code.push('('),
            SyntaxCategory::Item => self.code.push_str("const "),
            SyntaxCategory::Statement => {
                crate::ice::bug!("a statement placeholder is framed by its decision")
            }
        }
        self.push_placeholder_name(category, source, core_root)?;
        match category {
            SyntaxCategory::Expression => self.code.push(')'),
            SyntaxCategory::Propagation => self.code.push_str(");"),
            SyntaxCategory::Item => self.code.push_str(" = 0;"),
            SyntaxCategory::Statement => {
                crate::ice::bug!("a statement placeholder is framed by its decision")
            }
        }
        let owner_end = ProjectedByte(self.code.len());
        self.source_segments.push(ProjectionSourceSegment {
            projected: ProjectedSpan {
                start: owner_start,
                end: owner_end,
            },
            source,
            kind: ProjectionSegmentKind::Placeholder,
        });
        Ok(())
    }

    fn push_placeholder_name(
        &mut self,
        category: SyntaxCategory,
        source: SourceSpan,
        core_root: CoreRoot,
    ) -> Result<(), ProgramSyntaxError> {
        let ordinal =
            u32::try_from(self.pending.len()).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
        let id = TtNodeId(ordinal);
        let start = ProjectedByte(self.code.len());
        let prefix = match category {
            SyntaxCategory::Expression | SyntaxCategory::Propagation => "$tt_syntax_expr_",
            SyntaxCategory::Statement => "$tt_syntax_stmt_",
            SyntaxCategory::Item => "$tt_syntax_item_",
        };
        self.code.push_str(prefix);
        self.code.push_str(&ordinal.to_string());
        let end = ProjectedByte(self.code.len());
        self.pending.push(PendingOverlay {
            id,
            category,
            source,
            projected: ProjectedSpan { start, end },
            core_root,
            marker: OverlayMarker::Identifier,
            synthetic_return: None,
        });
        Ok(())
    }

    /// Writes the fixed delimiter after a source fragment embedded in a
    /// generated owner and records the source token after the fragment as
    /// the parse cause when SWC stops on that delimiter. This is
    /// provenance, not diagnostic-message inference: only a copied segment
    /// ending exactly at this boundary can own it.
    fn push_source_boundary(&mut self, text: &str, segments_since: usize) {
        let start = ProjectedByte(self.code.len());
        let source = self.source_segments[segments_since..]
            .iter()
            .rev()
            .find(|segment| {
                segment.kind == ProjectionSegmentKind::Copied && segment.projected.end == start
            })
            .map(|segment| self.token_after(segment.source.end));
        self.code.push_str(text);
        if let Some(source) = source {
            self.source_segments.push(ProjectionSourceSegment {
                projected: ProjectedSpan {
                    start,
                    end: ProjectedByte(self.code.len()),
                },
                source,
                kind: ProjectionSegmentKind::SourceBoundary,
            });
        }
    }

    fn token_after(&self, end: usize) -> SourceSpan {
        let start =
            crate::scanner::skip_ws_comments(self.source.as_bytes(), end, self.source.len());
        let width = self.source[start..]
            .chars()
            .next()
            .map_or(0, char::len_utf8);
        SourceSpan {
            start,
            end: start + width,
        }
    }

    fn emit_body(&mut self, body: BodyId) -> Result<(), ProgramSyntaxError> {
        crate::stack::grow(|| self.emit_body_grown(body))
    }

    fn emit_body_grown(&mut self, body: BodyId) -> Result<(), ProgramSyntaxError> {
        for statement in &self.core.bodies[body.index()].statements {
            if let Some(start) = self.statement_source_start(statement)? {
                self.preserve_statement_boundary(start);
            }
            match statement {
                Statement::Opaque(node) => self.push_source(*node)?,
                Statement::Adt(adt) => self.emit_adt(adt)?,
                Statement::Import(import) => self.emit_import(import)?,
                Statement::Propagate(propagate) => {
                    let start = ProjectedByte(self.code.len());
                    self.emit_propagate(propagate)?;
                    if let Some(binding) = &propagate.binding {
                        let declared = self.binding_identifier(binding.node)?;
                        self.completion.bindings.insert(
                            ProjectedSpan {
                                start,
                                end: ProjectedByte(self.code.len()),
                            },
                            declared,
                        );
                        let names = self.binding_text_names(binding.node)?;
                        self.tt_bindings.statements.push((
                            ProjectedSpan {
                                start,
                                end: ProjectedByte(self.code.len()),
                            },
                            names,
                        ));
                    }
                }
                Statement::Decision(decision) => {
                    let start = ProjectedByte(self.code.len());
                    self.emit_statement_decision(decision)?;
                    if let crate::core_ir::DecisionKind::LetElse { .. } = decision.kind {
                        let mut names = Vec::new();
                        for arm in &decision.arms {
                            self.pattern_names(&arm.pattern, &mut names)?;
                        }
                        self.tt_bindings.statements.push((
                            ProjectedSpan {
                                start,
                                end: ProjectedByte(self.code.len()),
                            },
                            names,
                        ));
                    }
                    if let crate::core_ir::DecisionKind::LetElse { exported: true, .. } =
                        decision.kind
                    {
                        self.code.push_str("export {};");
                    }
                }
                Statement::Expr(expr) => self.emit_expr(*expr)?,
            }
        }
        Ok(())
    }

    fn emit_adt(&mut self, adt: &Adt) -> Result<(), ProgramSyntaxError> {
        self.push_placeholder(
            SyntaxCategory::Item,
            self.source_span(adt.node)?,
            CoreRoot::Adt(adt.node),
        )
    }

    fn emit_import(&mut self, import: &Import) -> Result<(), ProgramSyntaxError> {
        self.push_source(import.specifier)
    }

    fn emit_propagate(&mut self, propagate: &Propagate) -> Result<(), ProgramSyntaxError> {
        // A declaration-form propagation can occur in a C-style `for`
        // initializer. Project it as an expression so that header remains
        // valid TypeScript; its typed continuation decides the eventual
        // statement shape in target lowering.
        if self.expr_contains_value_region(propagate.value) {
            return self.emit_propagate_with_shadow(
                SyntaxCategory::Propagation,
                self.source_span(propagate.owner)?,
                CoreRoot::Propagate(propagate.node),
                propagate.value,
            );
        }
        self.push_placeholder(
            SyntaxCategory::Propagation,
            self.source_span(propagate.owner)?,
            CoreRoot::Propagate(propagate.node),
        )
    }

    fn statement_source_start(
        &self,
        statement: &Statement,
    ) -> Result<Option<usize>, ProgramSyntaxError> {
        let node = match statement {
            Statement::Opaque(_) | Statement::Import(_) => return Ok(None),
            Statement::Adt(adt) => adt.node,
            Statement::Propagate(propagate) => propagate.owner,
            Statement::Decision(decision) => decision.extent,
            Statement::Expr(expr) => match &self.core.exprs[expr.index()] {
                Expr::Decision(decision) => decision.extent,
                Expr::Propagate(propagate) => propagate.node,
                Expr::Apply(apply) => {
                    let mut start = self.source_span(apply.node)?.start;
                    if let Some(head) = apply.head
                        && let Some(head_start) =
                            self.statement_source_start(&Statement::Expr(head))?
                    {
                        start = start.min(head_start);
                    }
                    return Ok(Some(start));
                }
                Expr::ResultRegion(region) => region.node,
                Expr::Opaque(_) | Expr::Sequence(_) | Expr::Template(_) => return Ok(None),
            },
        };
        Ok(Some(self.source_span(node)?.start))
    }

    fn preserve_statement_boundary(&mut self, source_start: usize) {
        if self
            .automatic_semicolons
            .binary_search_by_key(&source_start, |boundary| boundary.next)
            .is_ok()
        {
            let start = ProjectedByte(self.code.len());
            self.code.push(';');
            self.source_segments.push(ProjectionSourceSegment {
                projected: ProjectedSpan {
                    start,
                    end: ProjectedByte(self.code.len()),
                },
                source: SourceSpan {
                    start: source_start,
                    end: source_start,
                },
                kind: ProjectionSegmentKind::AutomaticSemicolon,
            });
        }
    }

    /// Keep the propagation as the statement's primary overlay while also
    /// exposing nested decisions to the TypeScript parent collector. The
    /// comma expression is projection-only; target lowering uses the two
    /// typed overlays to schedule the decision before the propagation.
    fn emit_propagate_with_shadow(
        &mut self,
        category: SyntaxCategory,
        source: SourceSpan,
        core_root: CoreRoot,
        value: ExprId,
    ) -> Result<(), ProgramSyntaxError> {
        let ordinal =
            u32::try_from(self.pending.len()).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
        let id = TtNodeId(ordinal);
        let pending_index = self.pending.len();
        self.pending.push(PendingOverlay {
            id,
            category,
            source,
            projected: ProjectedSpan {
                start: ProjectedByte(0),
                end: ProjectedByte(0),
            },
            core_root,
            marker: OverlayMarker::Identifier,
            synthetic_return: None,
        });
        let owner_start = ProjectedByte(self.code.len());
        self.code.push('(');
        self.emit_shadow_expr(value)?;
        self.code.push_str(", ");
        let start = ProjectedByte(self.code.len());
        self.code.push_str("$tt_syntax_expr_");
        self.code.push_str(&ordinal.to_string());
        let end = ProjectedByte(self.code.len());
        self.pending[pending_index].projected = ProjectedSpan { start, end };
        self.code.push(')');
        let owner_end = ProjectedByte(self.code.len());
        if category == SyntaxCategory::Propagation {
            self.code.push(';');
        }
        self.projection_only_protocol_parents.push(ProjectedSpan {
            start: ProjectedByte(owner_start.0 + 1),
            end: ProjectedByte(owner_end.0 - 1),
        });
        self.projection_only_protocol_parents.push(ProjectedSpan {
            start: owner_start,
            end: owner_end,
        });
        self.source_segments.push(ProjectionSourceSegment {
            projected: ProjectedSpan {
                start: ProjectedByte(owner_start.0 + 1),
                end: ProjectedByte(owner_end.0 - 1),
            },
            source,
            kind: ProjectionSegmentKind::Placeholder,
        });
        self.source_segments.push(ProjectionSourceSegment {
            projected: ProjectedSpan {
                start: owner_start,
                end: ProjectedByte(self.code.len()),
            },
            source,
            kind: ProjectionSegmentKind::Placeholder,
        });
        Ok(())
    }

    fn emit_statement_decision(&mut self, decision: &Decision) -> Result<(), ProgramSyntaxError> {
        crate::stack::grow(|| self.emit_statement_decision_grown(decision))
    }

    fn emit_statement_decision_grown(
        &mut self,
        decision: &Decision,
    ) -> Result<(), ProgramSyntaxError> {
        // The source decision is one statement, so its projection is one
        // block: as the unbraced body of an `if`, loop, or label, the
        // placeholder and the bodies below stay together under that parent,
        // and that block is the statement the decision's host owner maps to.
        let source = self.source_span(decision.extent)?;
        let segment_index = self.source_segments.len();
        let owner_start = ProjectedByte(self.code.len());
        self.code.push('{');
        self.push_placeholder_name(
            SyntaxCategory::Statement,
            source,
            CoreRoot::Decision(decision.extent),
        )?;
        self.code.push(';');
        for subject in &decision.subjects {
            self.code.push('(');
            let segments_since = self.source_segments.len();
            self.emit_expr(subject.value)?;
            self.push_source_boundary(");", segments_since);
        }
        // Statement decisions do not introduce a function boundary. Keep their
        // bodies in this lexical control-flow region so returns belong to the
        // surrounding match/result, and nested values retain their real owner.
        self.emit_inline_decision_bodies(decision)?;
        self.code.push('}');
        self.source_segments.insert(
            segment_index,
            ProjectionSourceSegment {
                projected: ProjectedSpan {
                    start: owner_start,
                    end: ProjectedByte(self.code.len()),
                },
                source,
                kind: ProjectionSegmentKind::Placeholder,
            },
        );
        Ok(())
    }

    fn emit_expr(&mut self, expr: ExprId) -> Result<(), ProgramSyntaxError> {
        crate::stack::grow(|| self.emit_expr_grown(expr))
    }

    fn emit_expr_grown(&mut self, expr: ExprId) -> Result<(), ProgramSyntaxError> {
        match &self.core.exprs[expr.index()] {
            Expr::Opaque(node) => self.push_source(*node),
            Expr::Sequence(body) => self.emit_body(*body),
            Expr::Decision(decision) => self.emit_decision_region(expr, decision),
            Expr::Propagate(propagate) => self.emit_propagate_expr(expr, propagate),
            Expr::Apply(apply) => self.emit_apply(expr, apply),
            Expr::ResultRegion(region) => self.emit_result_region(expr, region),
            Expr::Template(template) => self.emit_template(template),
        }
    }

    fn emit_propagate_expr(
        &mut self,
        expr: ExprId,
        propagate: &Propagate,
    ) -> Result<(), ProgramSyntaxError> {
        let source = self.source_span(propagate.node)?;
        if self.expr_contains_value_region(propagate.value) {
            return self.emit_propagate_with_shadow(
                SyntaxCategory::Expression,
                source,
                CoreRoot::Expr(expr),
                propagate.value,
            );
        }
        self.push_placeholder(SyntaxCategory::Expression, source, CoreRoot::Expr(expr))
    }

    fn emit_apply(&mut self, expr: ExprId, apply: &Apply) -> Result<(), ProgramSyntaxError> {
        let mut source = self.source_span(apply.node)?;
        for step in &apply.steps {
            let step_span = self.source_span(step.node)?;
            source.start = source.start.min(step_span.start);
            source.end = source.end.max(step_span.end);
        }
        // SWC has no pipeline syntax, so the pipeline itself remains one
        // expression placeholder.  A step can nevertheless contain a tt
        // value whose TypeScript evaluation structure the pipeline
        // placeholder hides. Project that step beside the placeholder in a
        // valid comma expression so the parent collector retains its arrow
        // or conditional boundary.
        if let Some(head) = apply.head
            && let Expr::Opaque(node) = &self.core.exprs[head.index()]
        {
            self.hidden_parts.push(HiddenPart {
                source: self.source_span(*node)?,
                role: HiddenRole::Head {
                    receiver: apply.steps.first().is_some_and(|step| {
                        matches!(step.mode, crate::core_ir::ApplyMode::Postfix { .. })
                    }),
                },
            });
        }
        for step in &apply.steps {
            if let Expr::Opaque(_) = &self.core.exprs[step.value.index()]
                && step.mode != crate::core_ir::ApplyMode::Missing
            {
                self.hidden_parts.push(HiddenPart {
                    source: self.source_span(step.node)?,
                    role: HiddenRole::Step {
                        postfix: matches!(step.mode, crate::core_ir::ApplyMode::Postfix { .. }),
                    },
                });
            }
        }
        let shadow_steps: Vec<_> = apply
            .steps
            .iter()
            .filter(|step| {
                self.expr_contains_propagation(step.value)
                    || self.expr_contains_value_region(step.value)
            })
            .collect();
        let shadow_head = apply.head.filter(|head| {
            self.expr_contains_propagation(*head) || self.expr_contains_value_region(*head)
        });
        if shadow_head.is_none() && shadow_steps.is_empty() {
            return self.push_placeholder(SyntaxCategory::Expression, source, CoreRoot::Expr(expr));
        }
        let start = ProjectedByte(self.code.len());
        self.code.push('(');
        self.push_placeholder(SyntaxCategory::Expression, source, CoreRoot::Expr(expr))?;
        if let Some(head) = shadow_head {
            self.code.push_str(", (");
            self.emit_shadow_expr(head)?;
            self.code.push(')');
        }
        for step in shadow_steps {
            self.code.push_str(", (");
            if apply.head.is_some()
                && matches!(step.mode, crate::core_ir::ApplyMode::Postfix { .. })
            {
                self.push_piped_value(self.source_span(step.node)?.start);
            }
            self.emit_shadow_expr(step.value)?;
            self.code.push(')');
        }
        self.code.push(')');
        self.projection_only_protocol_parents.push(ProjectedSpan {
            start: ProjectedByte(start.0 + 1),
            end: ProjectedByte(self.code.len() - 1),
        });
        self.projection_only_protocol_parents.push(ProjectedSpan {
            start,
            end: ProjectedByte(self.code.len()),
        });
        self.source_segments.push(ProjectionSourceSegment {
            projected: ProjectedSpan {
                // The comma-expression shadow is the projection of the
                // complete pipeline value. Include its grouping parens so
                // a host edge whose operand retains those parens (notably
                // an assignment RHS) maps to the pipeline's source span.
                start,
                end: ProjectedByte(self.code.len()),
            },
            source,
            kind: ProjectionSegmentKind::Placeholder,
        });
        Ok(())
    }

    fn push_piped_value(&mut self, step_start: usize) {
        let start = ProjectedByte(self.code.len());
        self.code.push_str("$tt_syntax_piped");
        self.source_segments.push(ProjectionSourceSegment {
            projected: ProjectedSpan {
                start,
                end: ProjectedByte(self.code.len()),
            },
            source: SourceSpan {
                start: step_start,
                end: step_start,
            },
            kind: ProjectionSegmentKind::Placeholder,
        });
    }

    fn expr_contains_propagation(&self, expr: ExprId) -> bool {
        crate::stack::grow(|| self.expr_contains_propagation_grown(expr))
    }

    fn expr_contains_propagation_grown(&self, expr: ExprId) -> bool {
        match &self.core.exprs[expr.index()] {
            Expr::Propagate(_) => true,
            Expr::Sequence(body) => self.body_contains_propagation(*body),
            Expr::Apply(apply) => apply
                .head
                .is_some_and(|head| self.expr_contains_propagation(head))
                || apply
                    .steps
                    .iter()
                    .any(|step| self.expr_contains_propagation(step.value)),
            Expr::Decision(decision) => decision.subjects.iter().any(|subject| {
                self.expr_contains_propagation(subject.value)
            }) || decision.arms.iter().any(|arm| {
                arm.guard.is_some_and(|guard| self.expr_contains_propagation(guard))
                    || matches!(arm.action, crate::core_ir::ArmAction::Yield { body, .. } | crate::core_ir::ArmAction::Execute(body) if self.core.bodies[body.index()].statements.iter().any(|statement| matches!(statement, Statement::Expr(expr) if self.expr_contains_propagation(*expr))))
            }),
            Expr::ResultRegion(region) => region.items.iter().any(|item| match item {
                crate::core_ir::ResultRegionItem::Statements(body) => {
                    self.body_contains_propagation(*body)
                }
            }) || region.value.is_some_and(|value| self.expr_contains_propagation(value)),
            Expr::Template(template) => template.parts.iter().any(|part| {
                matches!(part, TemplatePart::Interpolation(expr) if self.expr_contains_propagation(*expr))
            }),
            Expr::Opaque(_) => false,
        }
    }

    fn body_contains_propagation(&self, body: BodyId) -> bool {
        crate::stack::grow(|| self.body_contains_propagation_grown(body))
    }

    fn body_contains_propagation_grown(&self, body: BodyId) -> bool {
        self.core.bodies[body.index()]
            .statements
            .iter()
            .any(|statement| match statement {
                Statement::Propagate(_) => true,
                Statement::Expr(expr) => self.expr_contains_propagation(*expr),
                Statement::Decision(decision) => {
                    decision
                        .subjects
                        .iter()
                        .any(|subject| self.expr_contains_propagation(subject.value))
                        || decision.arms.iter().any(|arm| match arm.action {
                            crate::core_ir::ArmAction::Yield { body, .. }
                            | crate::core_ir::ArmAction::Execute(body) => {
                                self.body_contains_propagation(body)
                            }
                            crate::core_ir::ArmAction::BindThrough(_) => false,
                        })
                }
                Statement::Opaque(_) | Statement::Adt(_) | Statement::Import(_) => false,
            })
    }

    fn expr_contains_value_region(&self, expr: ExprId) -> bool {
        crate::stack::grow(|| self.expr_contains_value_region_grown(expr))
    }

    fn expr_contains_value_region_grown(&self, expr: ExprId) -> bool {
        match &self.core.exprs[expr.index()] {
            Expr::Decision(_) => true,
            Expr::Sequence(body) => self.core.bodies[body.index()].statements.iter().any(
                |statement| {
                    matches!(statement, Statement::Expr(expr) if self.expr_contains_value_region(*expr))
                },
            ),
            Expr::Apply(apply) => {
                apply
                    .head
                    .is_some_and(|head| self.expr_contains_value_region(head))
                    || apply
                        .steps
                        .iter()
                        .any(|step| self.expr_contains_value_region(step.value))
            }
            Expr::Propagate(propagate) => self.expr_contains_value_region(propagate.value),
            Expr::ResultRegion(_) => true,
            Expr::Template(template) => template.parts.iter().any(|part| {
                matches!(part, TemplatePart::Interpolation(expr) if self.expr_contains_value_region(*expr))
            }),
            Expr::Opaque(_) => false,
        }
    }

    fn emit_shadow_expr(&mut self, expr: ExprId) -> Result<(), ProgramSyntaxError> {
        match &self.core.exprs[expr.index()] {
            Expr::Opaque(node) => self.push_source(*node),
            Expr::Propagate(propagate) => self.emit_propagate_expr(expr, propagate),
            Expr::Sequence(body) => self.emit_shadow_body(*body),
            // A nested pipeline is opaque to SWC for the same reason as its
            // parent. Its own projection retains the structural placeholder.
            Expr::Apply(apply) => self.emit_apply(expr, apply),
            Expr::Decision(_) | Expr::ResultRegion(_) | Expr::Template(_) => self.emit_expr(expr),
        }
    }

    fn emit_shadow_body(&mut self, body: BodyId) -> Result<(), ProgramSyntaxError> {
        for statement in &self.core.bodies[body.index()].statements {
            match statement {
                Statement::Opaque(node) => self.push_source(*node)?,
                Statement::Expr(expr) => self.emit_shadow_expr(*expr)?,
                Statement::Propagate(propagate) => self.push_placeholder(
                    SyntaxCategory::Propagation,
                    self.source_span(propagate.owner)?,
                    CoreRoot::Propagate(propagate.node),
                )?,
                Statement::Adt(_) | Statement::Import(_) | Statement::Decision(_) => {
                    return Err(ProgramSyntaxError::InvalidSourceSpan {
                        start: SourceByte(0),
                        end: SourceByte(0),
                    });
                }
            }
        }
        Ok(())
    }

    fn emit_result_region(
        &mut self,
        expr: ExprId,
        region: &ResultRegion,
    ) -> Result<(), ProgramSyntaxError> {
        let source = self.source_span(region.node)?;
        let ordinal =
            u32::try_from(self.pending.len()).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
        let id = TtNodeId(ordinal);
        let start = ProjectedByte(self.code.len());
        let pending_index = self.pending.len();
        self.pending.push(PendingOverlay {
            id,
            category: SyntaxCategory::Expression,
            source,
            projected: ProjectedSpan { start, end: start },
            core_root: CoreRoot::Expr(expr),
            marker: OverlayMarker::CallExpression,
            synthetic_return: None,
        });
        self.push_region_function(region.is_async, region.in_generator);
        if let Some(labels) = &region.outward_jumps {
            for label in labels {
                self.code.push_str(label);
                self.code.push_str(": ");
            }
            self.code.push_str("for (;;) {");
        }
        self.code.push('{');
        let segments_since = self.source_segments.len();
        for item in &region.items {
            match item {
                crate::core_ir::ResultRegionItem::Statements(body) => self.emit_body(*body)?,
            }
        }
        self.push_source_boundary("}", segments_since);
        self.code.push('\n');
        let synthetic_return_start = ProjectedByte(self.code.len());
        self.code.push_str("return ");
        if let Some(value) = region.value {
            self.emit_expr(value)?;
        } else {
            self.code.push_str("undefined");
        }
        self.code.push(';');
        let synthetic_return_end = ProjectedByte(self.code.len());
        if region.outward_jumps.is_some() {
            self.code.push('}');
        }
        self.code.push_str("})()");
        let end = ProjectedByte(self.code.len());
        let projected = ProjectedSpan { start, end };
        self.source_segments.insert(
            0,
            ProjectionSourceSegment {
                projected,
                source,
                kind: ProjectionSegmentKind::Placeholder,
            },
        );
        self.pending[pending_index].projected = projected;
        self.pending[pending_index].synthetic_return = Some(ProjectedSpan {
            start: synthetic_return_start,
            end: synthetic_return_end,
        });
        Ok(())
    }

    fn push_region_function(&mut self, is_async: bool, in_generator: bool) -> ProjectedByte {
        self.code.push('(');
        let start = ProjectedByte(self.code.len());
        self.completion.regions.insert(start, Vec::new());
        if is_async {
            self.code.push_str("async ");
        }
        self.code.push_str(if in_generator {
            "function* () {"
        } else {
            "() => {"
        });
        start
    }

    fn emit_inline_decision_bodies(
        &mut self,
        decision: &Decision,
    ) -> Result<(), ProgramSyntaxError> {
        match &decision.kind {
            crate::core_ir::DecisionKind::LetElse { .. } => {
                let crate::core_ir::MissAction::Execute(body) = decision.miss else {
                    crate::ice::bug!("let-else has no else body")
                };
                self.code.push_str("if (true) {");
                self.emit_body(body)?;
                self.code.push('}');
            }
            crate::core_ir::DecisionKind::IfLet => {
                let crate::core_ir::ArmAction::Execute(body) = decision.arms[0].action else {
                    crate::ice::bug!("if-let has no then body")
                };
                self.code.push_str("if (true) ");
                let start = ProjectedByte(self.code.len());
                self.code.push('{');
                self.emit_body(body)?;
                self.code.push('}');
                let mut names = Vec::new();
                self.pattern_names(&decision.arms[0].pattern, &mut names)?;
                self.tt_bindings.scopes.push((
                    ProjectedSpan {
                        start,
                        end: ProjectedByte(self.code.len()),
                    },
                    names,
                ));
                match &decision.miss {
                    crate::core_ir::MissAction::Execute(body) => {
                        self.code.push_str(" else {");
                        self.emit_body(*body)?;
                        self.code.push('}');
                    }
                    crate::core_ir::MissAction::Decision(inner) => {
                        self.code.push_str(" else ");
                        self.emit_statement_decision(inner)?;
                    }
                    crate::core_ir::MissAction::Nothing => {}
                    crate::core_ir::MissAction::ThrowUnexpected(_) => {
                        crate::ice::bug!("if-let has match miss action")
                    }
                }
            }
            crate::core_ir::DecisionKind::Match { .. } => {
                crate::ice::bug!("expression decision in Result statement body")
            }
        }
        Ok(())
    }

    fn emit_decision_region(
        &mut self,
        expr: ExprId,
        decision: &Decision,
    ) -> Result<(), ProgramSyntaxError> {
        let source = self.source_span(decision.extent)?;
        let ordinal =
            u32::try_from(self.pending.len()).map_err(|_| ProgramSyntaxError::NodeCountOverflow)?;
        let id = TtNodeId(ordinal);
        let start = ProjectedByte(self.code.len());
        let pending_index = self.pending.len();
        self.pending.push(PendingOverlay {
            id,
            category: SyntaxCategory::Expression,
            source,
            projected: ProjectedSpan { start, end: start },
            core_root: CoreRoot::Expr(expr),
            marker: OverlayMarker::DecisionCallExpression,
            synthetic_return: None,
        });
        let region = self.push_region_function(decision.is_async, decision.in_generator);
        for subject in &decision.subjects {
            let host = ProjectedByte(self.code.len());
            self.code.push('(');
            let segments_since = self.source_segments.len();
            self.emit_expr(subject.value)?;
            self.push_source_boundary(");", segments_since);
            if let Some(hosts) = self.completion.regions.get_mut(&region) {
                hosts.push(ProjectedSpan {
                    start: host,
                    end: ProjectedByte(self.code.len()),
                });
            }
        }
        for arm in &decision.arms {
            let mut names = Vec::new();
            self.pattern_names(&arm.pattern, &mut names)?;
            let arm_start = ProjectedByte(self.code.len());
            self.emit_decision_arm(arm)?;
            self.tt_bindings.scopes.push((
                ProjectedSpan {
                    start: arm_start,
                    end: ProjectedByte(self.code.len()),
                },
                names,
            ));
        }
        self.code.push_str("0;})()");
        let end = ProjectedByte(self.code.len());
        let projected = ProjectedSpan { start, end };
        self.source_segments.insert(
            0,
            ProjectionSourceSegment {
                projected,
                source,
                kind: ProjectionSegmentKind::Placeholder,
            },
        );
        self.pending[pending_index].projected = projected;
        Ok(())
    }

    fn emit_decision_arm(
        &mut self,
        arm: &crate::core_ir::DecisionArm,
    ) -> Result<(), ProgramSyntaxError> {
        {
            if let Some(guard) = arm.guard {
                // A guard is evaluated only after its pattern bindings exist.
                // Map its projected statement as a complete owner so all of
                // its values share one evaluation plan, separate from the match.
                let start = ProjectedByte(self.code.len());
                self.code.push('(');
                let segments_since = self.source_segments.len();
                self.emit_expr(guard)?;
                self.push_source_boundary(");", segments_since);
                if let Expr::Sequence(body) = &self.core.exprs[guard.index()]
                    && let Some(node) = self.core.sequence_node(*body)
                {
                    self.source_segments.push(ProjectionSourceSegment {
                        projected: ProjectedSpan {
                            start,
                            end: ProjectedByte(self.code.len()),
                        },
                        source: self.source_span(node)?,
                        kind: ProjectionSegmentKind::Placeholder,
                    });
                }
            }
            let crate::core_ir::ArmAction::Yield { body, kind } = arm.action else {
                return Ok(());
            };
            match kind {
                hir::ArmBodyKind::Expression => {
                    self.code.push('(');
                    let segments_since = self.source_segments.len();
                    self.emit_body(body)?;
                    self.push_source_boundary(");", segments_since);
                }
                hir::ArmBodyKind::Missing => {}
                hir::ArmBodyKind::Block { .. } => {
                    let start = ProjectedByte(self.code.len());
                    self.code.push('{');
                    let segments_since = self.source_segments.len();
                    self.emit_body(body)?;
                    self.push_source_boundary("}", segments_since);
                    self.arm_blocks.insert(
                        ProjectedSpan {
                            start,
                            end: ProjectedByte(self.code.len()),
                        },
                        body,
                    );
                }
            }
        }
        Ok(())
    }

    fn pattern_names(
        &self,
        plan: &crate::core_ir::PatternPlan,
        names: &mut Vec<String>,
    ) -> Result<(), ProgramSyntaxError> {
        match plan {
            crate::core_ir::PatternPlan::Bind(bind) => {
                let span = self.source_span(bind.binding)?;
                names.push(self.source[span.start..span.end].to_owned());
            }
            crate::core_ir::PatternPlan::AllOf(parts)
            | crate::core_ir::PatternPlan::AnyOf(parts) => {
                for part in parts {
                    self.pattern_names(part, names)?;
                }
            }
            crate::core_ir::PatternPlan::Any | crate::core_ir::PatternPlan::Test(_) => {}
        }
        Ok(())
    }

    /// The name a declaration-form binding declares, when it is an
    /// identifier rather than a destructuring pattern.
    fn binding_identifier(&self, node: NodeId) -> Result<Option<String>, ProgramSyntaxError> {
        let span = self.source_span(node)?;
        let text = format!("({}) => 0", &self.source[span.start..span.end]);
        let input = crate::host_input::HostInput::new(&text);
        let mut parser = input.parser(crate::SourceKind::TypeScript);
        Ok(match parser.parse_expr().as_deref() {
            Ok(swc_ecma_ast::Expr::Arrow(arrow)) => match arrow.params.as_slice() {
                [Pat::Ident(binding)] => Some(binding.id.sym.to_string()),
                _ => None,
            },
            _ => None,
        })
    }

    fn binding_text_names(&self, node: NodeId) -> Result<Vec<String>, ProgramSyntaxError> {
        let span = self.source_span(node)?;
        let text = format!("({}) => 0", &self.source[span.start..span.end]);
        let input = crate::host_input::HostInput::new(&text);
        let mut parser = input.parser(crate::SourceKind::TypeScript);
        let mut names = Vec::new();
        if let Ok(expression) = parser.parse_expr()
            && let swc_ecma_ast::Expr::Arrow(arrow) = &*expression
        {
            for param in &arrow.params {
                scopes::pattern_names(param, &mut names);
            }
        }
        Ok(names)
    }

    fn emit_template(&mut self, template: &Template) -> Result<(), ProgramSyntaxError> {
        for part in &template.parts {
            match part {
                TemplatePart::Raw(node) => self.push_source(*node)?,
                TemplatePart::Interpolation(expr) => {
                    self.emit_expr(*expr)?;
                }
            }
        }
        Ok(())
    }
}
