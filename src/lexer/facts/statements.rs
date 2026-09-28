//! Statements, declarations, class bodies, and tt's statement-shaped
//! constructs (`if let`, let-else, `match` bodies, `variant` bodies).

use super::expressions::{ExprCfg, GroupKind, ObjectKind};
use super::types::TypeBody;
use super::{Frame, Machine, Out, Tk, Tok, TokenFacts, Yield, reserved};

/// A statement in progress. `start` is the byte where it began, for the
/// statement trace.
#[derive(Clone, Copy, Debug)]
pub(super) enum Stmt {
    /// An expression statement; its expression is the frame above.
    Expr {
        start: usize,
    },
    /// `return`/`throw`; `operand` once the operand is on the stack.
    Return {
        start: usize,
        operand: bool,
        restricted: bool,
    },
    /// `break`/`continue` (`label` while a label may follow) or `debugger`.
    Jump {
        start: usize,
        label: bool,
    },
    /// The statement's last part is done; it completes at the next token.
    Done {
        start: usize,
    },
    /// `if`: before its head, or after its consequent (`else` may follow).
    If {
        start: usize,
        then: bool,
    },
    /// `for`/`while`/`with` before the `(`.
    Head {
        start: usize,
        for_loop: bool,
    },
    /// Inside a `for` head.
    ForHead(ForHead),
    Do {
        start: usize,
        state: DoState,
    },
    Try {
        start: usize,
        state: TryState,
    },
    Switch {
        start: usize,
        head: bool,
    },
    /// A label, before its `:`.
    Label {
        start: usize,
    },
    /// Modifiers (`export`, `declare`, `async`, …) read; the statement
    /// continues with the next token.
    Modifier {
        start: usize,
    },
    /// Decorators read; the declaration follows.
    Decorated {
        start: usize,
    },
    /// `export`, before what it exports.
    Export {
        start: usize,
        default: bool,
    },
    /// An `import`/`export` clause.
    Module {
        start: usize,
        state: ModuleState,
    },
    /// `var`/`let`/`const`/`using` declarations, and tt let-else.
    Var(Var),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ForHead {
    Init,
    AfterInit,
    Test,
    Update,
    Rhs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DoState {
    Body,
    While,
    Cond,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TryState {
    Block,
    AfterBlock,
    CatchParam,
    CatchBody,
    AfterCatch,
    Finally,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ModuleState {
    Head,
    AfterClause,
    From,
    Source,
    Equals,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum VarState {
    Binding,
    AfterBinding,
    Init,
    ElseBlock,
    AfterElse,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Var {
    start: usize,
    state: VarState,
    /// A declaration in a `for` head, which the head's `;`, `of`, `in`, or
    /// `)` ends.
    for_head: bool,
    /// The binding is a tt pattern (`Some(v)`), whose declaration may take
    /// a let-else `else` block.
    pattern: bool,
}

impl Stmt {
    pub(super) fn start(&self) -> usize {
        match *self {
            Stmt::Expr { start }
            | Stmt::Return { start, .. }
            | Stmt::Jump { start, .. }
            | Stmt::Done { start }
            | Stmt::If { start, .. }
            | Stmt::Head { start, .. }
            | Stmt::Do { start, .. }
            | Stmt::Try { start, .. }
            | Stmt::Switch { start, .. }
            | Stmt::Label { start }
            | Stmt::Modifier { start }
            | Stmt::Decorated { start }
            | Stmt::Export { start, .. }
            | Stmt::Module { start, .. } => start,
            Stmt::Var(var) => var.start,
            Stmt::ForHead(_) => usize::MAX,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DeclKind {
    Function,
    Class,
    Interface,
    Alias,
    Enum,
    Namespace,
    Variant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DeclState {
    /// Right after the keyword.
    Keyword,
    /// After the name: type parameters, heritage, or the body.
    Head,
    /// After a function's parameters.
    Params,
    /// After a function's return type.
    Return,
    /// After `implements`/interface `extends` types.
    Types,
    /// After a type alias's `=` type.
    Value,
    /// The body is done.
    Done,
}

/// A declaration: a function or class (also in expression position, where
/// `start` is `None`), an interface, a type alias, an enum, a namespace or
/// module, or a tt `variant`.
#[derive(Clone, Copy, Debug)]
pub(super) struct Decl {
    start: Option<usize>,
    kind: DeclKind,
    state: DeclState,
    /// A `function*`.
    generator: bool,
}

impl Decl {
    pub(super) fn function(start: Option<usize>) -> Self {
        Decl {
            start,
            kind: DeclKind::Function,
            state: DeclState::Keyword,
            generator: false,
        }
    }

    pub(super) fn class(start: Option<usize>) -> Self {
        Decl {
            start,
            kind: DeclKind::Class,
            state: DeclState::Keyword,
            generator: false,
        }
    }

    fn new(start: usize, kind: DeclKind) -> Self {
        Decl {
            start: Some(start),
            kind,
            state: DeclState::Keyword,
            generator: false,
        }
    }

    pub(super) fn start(&self) -> Option<usize> {
        self.start
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TtIfState {
    Pattern,
    Scrutinee,
    AfterBody,
    Done,
}

/// A tt `if let` after its `if`: pattern, `=`, scrutinee, block, `else`.
#[derive(Clone, Copy, Debug)]
pub(super) struct TtIf {
    pub(super) start: Option<usize>,
    state: TtIfState,
    depth: u16,
}

impl TtIf {
    pub(super) fn new(start: Option<usize>) -> Self {
        TtIf {
            start,
            state: TtIfState::Pattern,
            depth: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SwitchBody {
    Clauses,
    Colon,
}

/// What kind of function a body `{` opens, for the facts on that brace.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum FunctionKind {
    #[default]
    Ordinary,
    Generator,
    Constructor,
}

/// A class body, positioned at one member; `function` is the kind of the
/// member's body if it has one.
#[derive(Clone, Copy, Debug)]
pub(super) struct ClassMember {
    state: ClassBody,
    function: FunctionKind,
}

impl ClassMember {
    pub(super) fn at(state: ClassBody) -> Self {
        ClassMember {
            state,
            function: FunctionKind::Ordinary,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ClassBody {
    Start,
    AfterName,
    AfterParams,
    AfterReturn,
    AfterType,
    AfterInit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchState {
    Pattern,
    Guard,
    Body,
    AfterBody,
}

/// A tt `match` body: `pattern [if guard] => body`, comma separated.
#[derive(Clone, Copy, Debug)]
pub(super) struct MatchBody {
    state: MatchState,
    depth: u16,
}

impl MatchBody {
    pub(super) fn new() -> Self {
        MatchBody {
            state: MatchState::Pattern,
            depth: 0,
        }
    }
}

impl Machine<'_> {
    /// Records on a `{` that it opens a body after `=>` or a parameter
    /// list, and which kind of function the body belongs to.
    pub(super) fn mark_function_body(&mut self, function: FunctionKind) {
        self.mark(TokenFacts::FUNCTION_BODY);
        match function {
            FunctionKind::Ordinary => {}
            FunctionKind::Generator => self.mark(TokenFacts::GENERATOR_BODY),
            FunctionKind::Constructor => self.mark(TokenFacts::CONSTRUCTOR_BODY),
        }
    }

    fn stmt_frame(&mut self, stmt: Stmt) {
        self.push_frame(Frame::Stmt(stmt));
    }

    fn slot_frame(&mut self) {
        self.push_frame(Frame::Slot);
    }

    fn statement_expr(&mut self) -> ExprCfg {
        ExprCfg {
            statement: true,
            ..ExprCfg::default()
        }
    }

    /// The end of a statement that closes with `;` or by automatic
    /// semicolon insertion, at the token after it.
    pub(super) fn end_statement(&mut self, start: usize, tok: &Tok<'_>) -> Out {
        if tok.is(b';') {
            self.record(start, tok.span.end);
            return Out::Consumed;
        }
        if tok.line_break && !tok.is(b'}') {
            self.mark(TokenFacts::ASI_BEFORE);
        }
        self.record(start, self.last_end);
        Out::Retry
    }

    fn expression_statement(&mut self, start: usize) -> Out {
        self.stmt_frame(Stmt::Expr { start });
        let cfg = self.statement_expr();
        self.push_expr(cfg);
        Out::Retry
    }

    /// Dispatches a token that begins a statement. `start` is where the
    /// statement began when modifiers or decorators came first; otherwise
    /// the token is the statement's first.
    pub(super) fn statement(&mut self, tok: &Tok<'_>, start: Option<usize>) -> Out {
        let begin = start.unwrap_or(tok.span.start);
        if start.is_none() {
            self.mark(TokenFacts::STATEMENT_START);
        }
        match tok.kind {
            Tk::Punct(b'{') => {
                self.push_frame(Frame::List {
                    closed: true,
                    block: Some(begin),
                    yields: Yield::Inherited,
                });
                Out::Consumed
            }
            Tk::Punct(b';') => {
                self.record(begin, tok.span.end);
                Out::Consumed
            }
            Tk::Punct(b')' | b']' | b'}' | b',' | b':') => Out::Consumed,
            Tk::Punct(b'@') => {
                self.stmt_frame(Stmt::Decorated { start: begin });
                self.push_frame(Frame::Decorator {
                    called: false,
                    name: false,
                });
                Out::Consumed
            }
            Tk::Word => self.statement_word(tok, begin, start.is_some()),
            _ => self.expression_statement(begin),
        }
    }

    fn statement_word(&mut self, tok: &Tok<'_>, begin: usize, modified: bool) -> Out {
        let peek = self.peek(tok.span.end);
        let next = self.byte(peek.at);
        let next_word = self.word_at(peek);
        let same_line_word = !peek.line_break && next_word.is_some();
        let same_line_name = same_line_word && !next_word.is_some_and(reserved);
        let var = |m: &mut Self| {
            m.stmt_frame(Stmt::Var(Var {
                start: begin,
                state: VarState::Binding,
                for_head: false,
                pattern: false,
            }));
            Out::Consumed
        };
        let modifier = |m: &mut Self| {
            m.stmt_frame(Stmt::Modifier { start: begin });
            Out::Consumed
        };
        let decl = |m: &mut Self, kind: DeclKind| {
            m.push_frame(Frame::Decl(Decl::new(begin, kind)));
            Out::Consumed
        };
        match tok.text {
            "const" if next_word == Some("enum") => modifier(self),
            "var" | "const" => var(self),
            "let" if next_word.is_some() || matches!(next, Some(b'[' | b'{')) => var(self),
            "using" if same_line_name => var(self),
            "await" if !peek.line_break && next_word == Some("using") => modifier(self),
            "async" if !peek.line_break && next_word == Some("function") => modifier(self),
            "abstract" if !peek.line_break && next_word == Some("class") => modifier(self),
            "declare" if same_line_word => modifier(self),
            "val" if !peek.line_break && matches!(next_word, Some("const" | "let" | "var")) => {
                modifier(self)
            }
            "function" => {
                self.push_frame(Frame::Decl(Decl::function(Some(begin))));
                Out::Consumed
            }
            "class" => {
                self.push_frame(Frame::Decl(Decl::class(Some(begin))));
                Out::Consumed
            }
            "interface" if same_line_name => decl(self, DeclKind::Interface),
            "type" if same_line_name => decl(self, DeclKind::Alias),
            "enum" => decl(self, DeclKind::Enum),
            "namespace" | "module"
                if !peek.line_break
                    && (next_word.is_some() || matches!(next, Some(b'"' | b'\''))) =>
            {
                decl(self, DeclKind::Namespace)
            }
            "global" if modified && next == Some(b'{') => {
                self.push_frame(Frame::Decl(Decl {
                    start: Some(begin),
                    kind: DeclKind::Namespace,
                    state: DeclState::Head,
                    generator: false,
                }));
                Out::Consumed
            }
            "variant" if same_line_name => decl(self, DeclKind::Variant),
            "export" => {
                self.stmt_frame(Stmt::Export {
                    start: begin,
                    default: false,
                });
                Out::Consumed
            }
            "import" if !matches!(next, Some(b'(' | b'.')) => {
                self.stmt_frame(Stmt::Module {
                    start: begin,
                    state: ModuleState::Head,
                });
                Out::Consumed
            }
            "if" => {
                if next == Some(b'(') {
                    self.stmt_frame(Stmt::If {
                        start: begin,
                        then: false,
                    });
                } else {
                    self.push_frame(Frame::TtIf(TtIf::new(Some(begin))));
                }
                Out::Consumed
            }
            "for" | "while" | "with" => {
                self.stmt_frame(Stmt::Head {
                    start: begin,
                    for_loop: tok.text == "for",
                });
                Out::Consumed
            }
            "do" => {
                self.stmt_frame(Stmt::Do {
                    start: begin,
                    state: DoState::Body,
                });
                self.slot_frame();
                Out::Consumed
            }
            "switch" => {
                self.stmt_frame(Stmt::Switch {
                    start: begin,
                    head: true,
                });
                Out::Consumed
            }
            "try" if next == Some(b'{') => {
                self.stmt_frame(Stmt::Try {
                    start: begin,
                    state: TryState::Block,
                });
                Out::Consumed
            }
            "return" | "throw" => {
                self.stmt_frame(Stmt::Return {
                    start: begin,
                    operand: false,
                    restricted: tok.text == "return",
                });
                Out::Consumed
            }
            "break" | "continue" | "debugger" => {
                self.stmt_frame(Stmt::Jump {
                    start: begin,
                    label: tok.text != "debugger",
                });
                Out::Consumed
            }
            "else" => {
                self.slot_frame();
                Out::Consumed
            }
            "case" | "default" | "catch" | "finally" => Out::Consumed,
            word if next == Some(b':')
                && self.byte(peek.at + 1) != Some(b':')
                && !reserved(word) =>
            {
                self.mark(TokenFacts::LABEL);
                self.stmt_frame(Stmt::Label { start: begin });
                Out::Consumed
            }
            _ => self.expression_statement(begin),
        }
    }

    pub(super) fn stmt(&mut self, stmt: Stmt, tok: &Tok<'_>) -> Out {
        match stmt {
            Stmt::Expr { start } => self.end_statement(start, tok),
            Stmt::Return {
                start,
                operand,
                restricted,
            } => {
                if operand {
                    return self.end_statement(start, tok);
                }
                if (restricted && tok.line_break) || matches!(tok.kind, Tk::Punct(b';' | b'}')) {
                    return self.end_statement(start, tok);
                }
                self.stmt_frame(Stmt::Return {
                    start,
                    operand: true,
                    restricted,
                });
                let cfg = self.statement_expr();
                self.push_expr(cfg);
                Out::Retry
            }
            Stmt::Jump { start, label } => {
                if label && tok.kind == Tk::Word && !tok.line_break && !reserved(tok.text) {
                    self.mark(TokenFacts::LABEL);
                    self.stmt_frame(Stmt::Jump {
                        start,
                        label: false,
                    });
                    return Out::Consumed;
                }
                self.end_statement(start, tok)
            }
            Stmt::Done { start } => {
                self.record(start, self.last_end);
                Out::Retry
            }
            Stmt::If { start, then: false } => {
                self.stmt_frame(Stmt::If { start, then: true });
                self.slot_frame();
                if tok.is(b'(') {
                    self.open_group(GroupKind::Control);
                    Out::Consumed
                } else {
                    Out::Retry
                }
            }
            Stmt::If { start, then: true } => {
                if tok.is_word("else") {
                    self.stmt_frame(Stmt::Done { start });
                    self.slot_frame();
                    return Out::Consumed;
                }
                self.record(start, self.last_end);
                Out::Retry
            }
            Stmt::Head { start, for_loop } => {
                if for_loop && tok.is_word("await") {
                    self.stmt_frame(stmt);
                    return Out::Consumed;
                }
                self.stmt_frame(Stmt::Done { start });
                self.slot_frame();
                if !tok.is(b'(') {
                    return Out::Retry;
                }
                if for_loop {
                    self.stmt_frame(Stmt::ForHead(ForHead::Init));
                } else {
                    self.open_group(GroupKind::Control);
                }
                Out::Consumed
            }
            Stmt::ForHead(state) => self.for_head(state, tok),
            Stmt::Do { start, state } => match state {
                DoState::Body if tok.is_word("while") => {
                    self.stmt_frame(Stmt::Do {
                        start,
                        state: DoState::While,
                    });
                    Out::Consumed
                }
                DoState::While if tok.is(b'(') => {
                    self.stmt_frame(Stmt::Do {
                        start,
                        state: DoState::Cond,
                    });
                    self.open_group(GroupKind::Control);
                    Out::Consumed
                }
                DoState::Cond if tok.is(b';') => {
                    self.record(start, tok.span.end);
                    Out::Consumed
                }
                _ => {
                    self.record(start, self.last_end);
                    Out::Retry
                }
            },
            Stmt::Try { start, state } => self.try_statement(start, state, tok),
            Stmt::Switch { start, head } => {
                if head && tok.is(b'(') {
                    self.stmt_frame(Stmt::Switch { start, head: false });
                    self.open_group(GroupKind::Control);
                    return Out::Consumed;
                }
                if !head && tok.is(b'{') {
                    self.stmt_frame(Stmt::Done { start });
                    self.push_frame(Frame::Switch(SwitchBody::Clauses));
                    return Out::Consumed;
                }
                self.end_statement(start, tok)
            }
            Stmt::Label { start } => {
                if tok.is(b':') {
                    self.stmt_frame(Stmt::Done { start });
                    self.slot_frame();
                    return Out::Consumed;
                }
                self.end_statement(start, tok)
            }
            Stmt::Modifier { start } => self.statement(tok, Some(start)),
            Stmt::Decorated { start } => {
                if tok.is(b'@') {
                    self.stmt_frame(stmt);
                    self.push_frame(Frame::Decorator {
                        called: false,
                        name: false,
                    });
                    return Out::Consumed;
                }
                self.statement(tok, Some(start))
            }
            Stmt::Export { start, default } => self.export(start, default, tok),
            Stmt::Module { start, state } => self.module_item(start, state, tok),
            Stmt::Var(var) => self.var(var, tok),
        }
    }

    fn for_head(&mut self, state: ForHead, tok: &Tok<'_>) -> Out {
        let expr = ExprCfg::default();
        match state {
            ForHead::Init => {
                if tok.is(b')') {
                    return Out::Consumed;
                }
                if tok.is(b';') {
                    self.stmt_frame(Stmt::ForHead(ForHead::Test));
                    self.push_expr(expr);
                    return Out::Consumed;
                }
                let (next, next_word, next_break) = self.next_after(tok);
                let declaration = match tok.word() {
                    Some("var" | "const") => true,
                    Some("let") => next_word.is_some() || matches!(next, Some(b'[' | b'{')),
                    Some("using") => !next_break && next_word.is_some() && next_word != Some("of"),
                    _ => false,
                };
                self.stmt_frame(Stmt::ForHead(ForHead::AfterInit));
                if declaration {
                    self.stmt_frame(Stmt::Var(Var {
                        start: tok.span.start,
                        state: VarState::Binding,
                        for_head: true,
                        pattern: false,
                    }));
                    return Out::Consumed;
                }
                if tok.is_word("await") && next_word == Some("using") && !next_break {
                    self.stmt_frame(Stmt::ForHead(ForHead::Init));
                    return Out::Consumed;
                }
                self.push_expr(expr);
                Out::Retry
            }
            ForHead::AfterInit => {
                if tok.is(b')') {
                    return Out::Consumed;
                }
                if tok.is_word("of") || tok.is_word("in") {
                    self.stmt_frame(Stmt::ForHead(ForHead::Rhs));
                    self.push_expr(expr);
                    return Out::Consumed;
                }
                if tok.is(b';') {
                    self.stmt_frame(Stmt::ForHead(ForHead::Test));
                    self.push_expr(expr);
                    return Out::Consumed;
                }
                self.stmt_frame(Stmt::ForHead(state));
                Out::Consumed
            }
            ForHead::Test | ForHead::Update | ForHead::Rhs => {
                if tok.is(b')') {
                    return Out::Consumed;
                }
                if tok.is(b';') && state == ForHead::Test {
                    self.stmt_frame(Stmt::ForHead(ForHead::Update));
                    self.push_expr(expr);
                    return Out::Consumed;
                }
                if matches!(tok.kind, Tk::Punct(b'}' | b']')) {
                    return Out::Retry;
                }
                self.stmt_frame(Stmt::ForHead(state));
                if matches!(tok.kind, Tk::Punct(b';' | b',' | b':')) {
                    return Out::Consumed;
                }
                self.push_expr(expr);
                Out::Retry
            }
        }
    }

    fn try_statement(&mut self, start: usize, state: TryState, tok: &Tok<'_>) -> Out {
        let keep = |m: &mut Self, state: TryState| m.stmt_frame(Stmt::Try { start, state });
        match state {
            TryState::Block | TryState::CatchBody | TryState::Finally if tok.is(b'{') => {
                let after = match state {
                    TryState::Block => Some(TryState::AfterBlock),
                    TryState::CatchBody => Some(TryState::AfterCatch),
                    _ => None,
                };
                match after {
                    Some(after) => keep(self, after),
                    None => self.stmt_frame(Stmt::Done { start }),
                }
                self.open_block();
                Out::Consumed
            }
            TryState::AfterBlock | TryState::AfterCatch if tok.is_word("finally") => {
                keep(self, TryState::Finally);
                Out::Consumed
            }
            TryState::AfterBlock if tok.is_word("catch") => {
                keep(self, TryState::CatchParam);
                Out::Consumed
            }
            TryState::CatchParam if tok.is(b'(') => {
                keep(self, TryState::CatchBody);
                self.open_params();
                Out::Consumed
            }
            TryState::CatchParam if tok.is(b'{') => {
                keep(self, TryState::CatchBody);
                Out::Retry
            }
            _ => {
                self.record(start, self.last_end);
                Out::Retry
            }
        }
    }

    fn export(&mut self, start: usize, default: bool, tok: &Tok<'_>) -> Out {
        let (next, _, _) = self.next_after(tok);
        match tok.kind {
            Tk::Word if !default && tok.text == "default" => {
                self.stmt_frame(Stmt::Export {
                    start,
                    default: true,
                });
                Out::Consumed
            }
            Tk::Punct(b'{' | b'*') if !default => {
                self.stmt_frame(Stmt::Module {
                    start,
                    state: ModuleState::Head,
                });
                Out::Retry
            }
            Tk::Punct(b'=') if !default => {
                self.stmt_frame(Stmt::Expr { start });
                let cfg = self.statement_expr();
                self.push_expr(cfg);
                Out::Consumed
            }
            Tk::Word
                if !default
                    && (matches!(tok.text, "import" | "as")
                        || (tok.text == "type" && matches!(next, Some(b'{' | b'*')))) =>
            {
                self.stmt_frame(Stmt::Module {
                    start,
                    state: ModuleState::Head,
                });
                Out::Consumed
            }
            Tk::Word
                if default
                    && !matches!(
                        tok.text,
                        "function" | "class" | "interface" | "abstract" | "async" | "enum"
                    ) =>
            {
                self.expression_statement(start)
            }
            Tk::Word => self.statement(tok, Some(start)),
            _ if default => self.expression_statement(start),
            _ => self.statement(tok, Some(start)),
        }
    }

    fn module_item(&mut self, start: usize, state: ModuleState, tok: &Tok<'_>) -> Out {
        let keep = |m: &mut Self, state: ModuleState| m.stmt_frame(Stmt::Module { start, state });
        if tok.is(b';') || tok.is(b'}') {
            return self.end_statement(start, tok);
        }
        match state {
            ModuleState::Head => match tok.kind {
                Tk::Punct(b'{') => {
                    keep(self, ModuleState::AfterClause);
                    self.open_object(ObjectKind::Literal);
                    Out::Consumed
                }
                Tk::Punct(b'*') => {
                    keep(self, ModuleState::AfterClause);
                    Out::Consumed
                }
                Tk::Str => {
                    keep(self, ModuleState::Source);
                    Out::Consumed
                }
                Tk::Punct(b'=') => {
                    keep(self, ModuleState::Equals);
                    let cfg = self.statement_expr();
                    self.push_expr(cfg);
                    Out::Consumed
                }
                Tk::Word if matches!(tok.text, "type" | "typeof") => {
                    keep(self, ModuleState::Head);
                    Out::Consumed
                }
                Tk::Word => {
                    keep(self, ModuleState::AfterClause);
                    Out::Consumed
                }
                _ => self.end_statement(start, tok),
            },
            ModuleState::AfterClause => match tok.kind {
                Tk::Punct(b',') => {
                    keep(self, ModuleState::Head);
                    Out::Consumed
                }
                Tk::Punct(b'=') => {
                    keep(self, ModuleState::Head);
                    Out::Retry
                }
                Tk::Word if tok.text == "from" => {
                    keep(self, ModuleState::From);
                    Out::Consumed
                }
                Tk::Word if tok.text == "as" => {
                    keep(self, ModuleState::Head);
                    Out::Consumed
                }
                Tk::Word if !tok.line_break => {
                    keep(self, ModuleState::AfterClause);
                    Out::Consumed
                }
                _ => self.end_statement(start, tok),
            },
            ModuleState::From if tok.kind == Tk::Str => {
                keep(self, ModuleState::Source);
                Out::Consumed
            }
            ModuleState::Source if matches!(tok.word(), Some("with" | "assert")) => {
                keep(self, ModuleState::Source);
                Out::Consumed
            }
            ModuleState::Source if tok.is(b'{') && !tok.line_break => {
                keep(self, ModuleState::Source);
                self.open_object(ObjectKind::Literal);
                Out::Consumed
            }
            _ => self.end_statement(start, tok),
        }
    }

    fn var(&mut self, mut var: Var, tok: &Tok<'_>) -> Out {
        let ends_head = var.for_head
            && (tok.is_word("of") || tok.is_word("in") || tok.is(b';') || tok.is(b')'));
        let keep = |m: &mut Self, var: Var| m.stmt_frame(Stmt::Var(var));
        match var.state {
            VarState::Binding => match tok.kind {
                Tk::Word => {
                    var.state = VarState::AfterBinding;
                    keep(self, var);
                    Out::Consumed
                }
                Tk::Punct(b'{') => {
                    var.state = VarState::AfterBinding;
                    keep(self, var);
                    self.open_object(ObjectKind::Literal);
                    Out::Consumed
                }
                Tk::Punct(b'[') => {
                    var.state = VarState::AfterBinding;
                    keep(self, var);
                    self.open_group(GroupKind::Array);
                    Out::Consumed
                }
                _ if var.for_head => Out::Retry,
                _ => self.end_statement(var.start, tok),
            },
            VarState::AfterBinding => match tok.kind {
                Tk::Punct(b'(') if !tok.line_break => {
                    var.pattern = true;
                    keep(self, var);
                    self.open_group(GroupKind::Grouping);
                    Out::Consumed
                }
                Tk::Punct(b'|') => {
                    var.state = VarState::Binding;
                    keep(self, var);
                    Out::Consumed
                }
                Tk::Punct(b'!') => {
                    keep(self, var);
                    Out::Consumed
                }
                Tk::Punct(b':') => {
                    keep(self, var);
                    self.open_type();
                    Out::Consumed
                }
                _ => self.var_tail(var, tok, ends_head),
            },
            VarState::Init => self.var_tail(var, tok, ends_head),
            VarState::ElseBlock if tok.is(b'{') => {
                var.state = VarState::AfterElse;
                keep(self, var);
                self.open_block();
                Out::Consumed
            }
            VarState::ElseBlock | VarState::AfterElse => self.end_statement(var.start, tok),
        }
    }

    fn var_tail(&mut self, mut var: Var, tok: &Tok<'_>, ends_head: bool) -> Out {
        if ends_head {
            return Out::Retry;
        }
        match tok.kind {
            Tk::Punct(b'=') if var.state == VarState::AfterBinding => {
                var.state = VarState::Init;
                self.stmt_frame(Stmt::Var(var));
                self.push_expr(ExprCfg {
                    stop_comma: true,
                    statement: !var.for_head,
                    ..ExprCfg::default()
                });
                Out::Consumed
            }
            Tk::Punct(b',') => {
                var.state = VarState::Binding;
                self.stmt_frame(Stmt::Var(var));
                Out::Consumed
            }
            Tk::Word if var.pattern && tok.text == "else" => {
                var.state = VarState::ElseBlock;
                self.stmt_frame(Stmt::Var(var));
                Out::Consumed
            }
            _ if var.for_head => Out::Retry,
            _ => self.end_statement(var.start, tok),
        }
    }

    pub(super) fn decl(&mut self, mut decl: Decl, tok: &Tok<'_>) -> Out {
        let keep = |m: &mut Self, decl: Decl| m.push_frame(Frame::Decl(decl));
        if decl.state == DeclState::Done {
            if let Some(start) = decl.start {
                self.record(start, self.last_end);
            }
            return Out::Retry;
        }
        match (decl.kind, decl.state, tok.kind) {
            (DeclKind::Function, DeclState::Keyword, Tk::Punct(b'*')) => {
                decl.generator = true;
                keep(self, decl);
                Out::Consumed
            }
            (_, DeclState::Keyword, Tk::Word)
                if !(decl.kind == DeclKind::Class
                    && matches!(tok.text, "extends" | "implements")) =>
            {
                decl.state = DeclState::Head;
                keep(self, decl);
                Out::Consumed
            }
            (DeclKind::Namespace, DeclState::Keyword | DeclState::Head, Tk::Str) => {
                decl.state = DeclState::Head;
                keep(self, decl);
                Out::Consumed
            }
            (DeclKind::Namespace, DeclState::Head, Tk::Punct(b'.')) => {
                decl.state = DeclState::Keyword;
                keep(self, decl);
                Out::Consumed
            }
            (_, DeclState::Keyword | DeclState::Head, Tk::Punct(b'<')) => {
                decl.state = DeclState::Head;
                keep(self, decl);
                self.open_type_group(b'>');
                Out::Consumed
            }
            (DeclKind::Function, DeclState::Keyword | DeclState::Head, Tk::Punct(b'(')) => {
                decl.state = DeclState::Params;
                keep(self, decl);
                self.open_params();
                Out::Consumed
            }
            (DeclKind::Function, DeclState::Params, Tk::Punct(b':')) => {
                decl.state = DeclState::Return;
                keep(self, decl);
                self.open_type();
                Out::Consumed
            }
            (DeclKind::Function, DeclState::Params | DeclState::Return, Tk::Punct(b'{')) => {
                decl.state = DeclState::Done;
                keep(self, decl);
                self.open_function_body(if decl.generator {
                    FunctionKind::Generator
                } else {
                    FunctionKind::Ordinary
                });
                Out::Consumed
            }
            (DeclKind::Class, DeclState::Keyword | DeclState::Head, Tk::Word)
                if tok.text == "extends" =>
            {
                decl.state = DeclState::Head;
                keep(self, decl);
                self.push_expr(ExprCfg {
                    heritage: true,
                    brace_ends: true,
                    ..ExprCfg::default()
                });
                Out::Consumed
            }
            (DeclKind::Class | DeclKind::Interface, _, Tk::Word)
                if matches!(tok.text, "implements" | "extends") =>
            {
                decl.state = DeclState::Types;
                keep(self, decl);
                self.open_type();
                Out::Consumed
            }
            (DeclKind::Class | DeclKind::Interface, DeclState::Types, Tk::Punct(b',')) => {
                keep(self, decl);
                self.open_type();
                Out::Consumed
            }
            (
                DeclKind::Class,
                DeclState::Keyword | DeclState::Head | DeclState::Types,
                Tk::Punct(b'{'),
            ) => {
                decl.state = DeclState::Done;
                keep(self, decl);
                self.push_frame(Frame::ClassBody(ClassMember::at(ClassBody::Start)));
                Out::Consumed
            }
            (DeclKind::Interface, DeclState::Head | DeclState::Types, Tk::Punct(b'{')) => {
                decl.state = DeclState::Done;
                keep(self, decl);
                self.push_frame(Frame::TypeBody(TypeBody::Start));
                Out::Consumed
            }
            (DeclKind::Alias, DeclState::Head, Tk::Punct(b'=')) => {
                decl.state = DeclState::Value;
                keep(self, decl);
                self.open_type();
                Out::Consumed
            }
            (DeclKind::Enum | DeclKind::Variant, DeclState::Head, Tk::Punct(b'{')) => {
                decl.state = DeclState::Done;
                keep(self, decl);
                self.open_object(if decl.kind == DeclKind::Enum {
                    ObjectKind::Enum
                } else {
                    ObjectKind::Variant
                });
                Out::Consumed
            }
            (DeclKind::Namespace, DeclState::Head, Tk::Punct(b'{')) => {
                decl.state = DeclState::Done;
                keep(self, decl);
                self.open_body(Yield::Identifier);
                Out::Consumed
            }
            _ => match decl.start {
                Some(start) => self.end_statement(start, tok),
                None => Out::Retry,
            },
        }
    }

    pub(super) fn tt_if(&mut self, mut tt: TtIf, tok: &Tok<'_>) -> Out {
        let end = |m: &mut Self, tt: TtIf, tok: &Tok<'_>| match tt.start {
            Some(start) => m.end_statement(start, tok),
            None => Out::Retry,
        };
        match tt.state {
            TtIfState::Pattern => {
                match tok.kind {
                    Tk::Punct(b'=') if tt.depth == 0 && self.byte(tok.span.end) != Some(b'=') => {
                        tt.state = TtIfState::Scrutinee;
                        self.push_frame(Frame::TtIf(tt));
                        self.push_expr(ExprCfg {
                            brace_ends: true,
                            statement: true,
                            ..ExprCfg::default()
                        });
                        return Out::Consumed;
                    }
                    Tk::Punct(b'(' | b'[' | b'{') => tt.depth += 1,
                    Tk::Punct(b')' | b']' | b'}') if tt.depth > 0 => tt.depth -= 1,
                    Tk::Punct(b')' | b']' | b'}' | b';') => return end(self, tt, tok),
                    _ => {}
                }
                self.push_frame(Frame::TtIf(tt));
                Out::Consumed
            }
            TtIfState::Scrutinee if tok.is(b'{') => {
                tt.state = TtIfState::AfterBody;
                self.push_frame(Frame::TtIf(tt));
                self.open_block();
                Out::Consumed
            }
            TtIfState::Scrutinee => end(self, tt, tok),
            TtIfState::AfterBody if tok.is_word("else") => {
                tt.state = TtIfState::Done;
                self.push_frame(Frame::TtIf(tt));
                self.slot_frame();
                Out::Consumed
            }
            TtIfState::AfterBody | TtIfState::Done => {
                if let Some(start) = tt.start {
                    self.record(start, self.last_end);
                }
                Out::Retry
            }
        }
    }

    pub(super) fn switch_body(&mut self, state: SwitchBody, tok: &Tok<'_>) -> Out {
        if tok.is(b'}') {
            return Out::Consumed;
        }
        match state {
            SwitchBody::Colon => {
                self.push_frame(Frame::Switch(SwitchBody::Clauses));
                if tok.is(b':') {
                    Out::Consumed
                } else {
                    Out::Retry
                }
            }
            SwitchBody::Clauses => match tok.word() {
                Some("case") => {
                    self.push_frame(Frame::Switch(SwitchBody::Colon));
                    self.push_expr(ExprCfg {
                        colon_ends: true,
                        ..ExprCfg::default()
                    });
                    Out::Consumed
                }
                Some("default") => {
                    self.push_frame(Frame::Switch(SwitchBody::Colon));
                    Out::Consumed
                }
                _ => {
                    self.push_frame(Frame::Switch(SwitchBody::Clauses));
                    self.statement(tok, None)
                }
            },
        }
    }

    pub(super) fn class_body(&mut self, member: ClassMember, tok: &Tok<'_>) -> Out {
        let keep = |m: &mut Self, state: ClassBody| {
            let function = if state == ClassBody::Start {
                FunctionKind::Ordinary
            } else {
                member.function
            };
            m.push_frame(Frame::ClassBody(ClassMember { state, function }));
        };
        if tok.is(b'}') {
            return Out::Consumed;
        }
        match member.state {
            ClassBody::Start => match tok.kind {
                Tk::Punct(b'*') => {
                    self.push_frame(Frame::ClassBody(ClassMember {
                        state: ClassBody::Start,
                        function: FunctionKind::Generator,
                    }));
                    Out::Consumed
                }
                Tk::Punct(b';' | b'#') => {
                    self.push_frame(Frame::ClassBody(member));
                    Out::Consumed
                }
                Tk::Punct(b'@') => {
                    self.push_frame(Frame::ClassBody(member));
                    self.push_frame(Frame::Decorator {
                        called: false,
                        name: false,
                    });
                    Out::Consumed
                }
                Tk::Punct(b'{') => {
                    keep(self, ClassBody::Start);
                    self.open_body(Yield::Identifier);
                    Out::Consumed
                }
                Tk::Punct(b'[') => {
                    keep(self, ClassBody::AfterName);
                    self.open_group(GroupKind::Computed);
                    Out::Consumed
                }
                Tk::Word => {
                    let (next, next_word, next_break) = self.next_after(tok);
                    let named_next = next_word.is_some()
                        || matches!(next, Some(b'[' | b'"' | b'\'' | b'#' | b'*'))
                        || next.is_some_and(|byte| byte.is_ascii_digit());
                    let modifier = match tok.text {
                        "static" => named_next || next == Some(b'{'),
                        "get" | "set" => named_next,
                        "public" | "private" | "protected" | "readonly" | "abstract"
                        | "declare" | "override" | "accessor" | "async" => {
                            named_next && !next_break
                        }
                        _ => false,
                    };
                    if modifier {
                        self.push_frame(Frame::ClassBody(member));
                    } else {
                        self.mark(TokenFacts::MEMBER);
                        self.push_frame(Frame::ClassBody(ClassMember {
                            state: ClassBody::AfterName,
                            function: if tok.text == "constructor" {
                                FunctionKind::Constructor
                            } else {
                                member.function
                            },
                        }));
                    }
                    Out::Consumed
                }
                Tk::Str | Tk::Number => {
                    self.mark(TokenFacts::MEMBER);
                    keep(self, ClassBody::AfterName);
                    Out::Consumed
                }
                _ => {
                    self.push_frame(Frame::ClassBody(member));
                    Out::Consumed
                }
            },
            ClassBody::AfterName => match tok.kind {
                Tk::Punct(b'?' | b'!') => {
                    keep(self, ClassBody::AfterName);
                    Out::Consumed
                }
                Tk::Punct(b'(') => {
                    keep(self, ClassBody::AfterParams);
                    self.open_params();
                    Out::Consumed
                }
                Tk::Punct(b'<') => {
                    keep(self, ClassBody::AfterName);
                    self.open_type_group(b'>');
                    Out::Consumed
                }
                Tk::Punct(b':') => {
                    keep(self, ClassBody::AfterType);
                    self.open_type();
                    Out::Consumed
                }
                _ => self.class_member_tail(tok),
            },
            ClassBody::AfterType => self.class_member_tail(tok),
            ClassBody::AfterParams | ClassBody::AfterReturn => match tok.kind {
                Tk::Punct(b':') if member.state == ClassBody::AfterParams => {
                    keep(self, ClassBody::AfterReturn);
                    self.open_type();
                    Out::Consumed
                }
                Tk::Punct(b'{') => {
                    keep(self, ClassBody::Start);
                    self.open_function_body(member.function);
                    Out::Consumed
                }
                _ => {
                    keep(self, ClassBody::Start);
                    if tok.is(b';') {
                        Out::Consumed
                    } else {
                        Out::Retry
                    }
                }
            },
            ClassBody::AfterInit => {
                keep(self, ClassBody::Start);
                if tok.is(b';') {
                    Out::Consumed
                } else {
                    Out::Retry
                }
            }
        }
    }

    fn class_member_tail(&mut self, tok: &Tok<'_>) -> Out {
        if tok.is(b'=') {
            self.push_frame(Frame::ClassBody(ClassMember::at(ClassBody::AfterInit)));
            self.push_expr(ExprCfg::default());
            return Out::Consumed;
        }
        self.push_frame(Frame::ClassBody(ClassMember::at(ClassBody::Start)));
        if tok.is(b';') {
            Out::Consumed
        } else {
            Out::Retry
        }
    }

    pub(super) fn match_body(&mut self, mut body: MatchBody, tok: &Tok<'_>) -> Out {
        let keep = |m: &mut Self, body: MatchBody| m.push_frame(Frame::Match(body));
        match body.state {
            MatchState::Pattern => {
                match tok.kind {
                    Tk::Punct(b'}') if body.depth == 0 => return Out::Consumed,
                    Tk::Arrow if body.depth == 0 => {
                        body.state = MatchState::Body;
                    }
                    Tk::Word if body.depth == 0 && tok.text == "if" => {
                        body.state = MatchState::Guard;
                        keep(self, body);
                        self.push_expr(ExprCfg {
                            arrow_ends: true,
                            ..ExprCfg::default()
                        });
                        return Out::Consumed;
                    }
                    Tk::Punct(b'(' | b'[' | b'{') => body.depth += 1,
                    Tk::Punct(b')' | b']' | b'}') => body.depth = body.depth.saturating_sub(1),
                    _ => {}
                }
                keep(self, body);
                Out::Consumed
            }
            MatchState::Guard => {
                if tok.kind == Tk::Arrow {
                    body.state = MatchState::Body;
                    keep(self, body);
                    return Out::Consumed;
                }
                if tok.is(b'}') {
                    return Out::Consumed;
                }
                body.state = MatchState::Pattern;
                keep(self, body);
                Out::Retry
            }
            MatchState::Body => {
                if tok.is(b'}') {
                    return Out::Consumed;
                }
                body.state = MatchState::AfterBody;
                keep(self, body);
                if tok.is(b'{') {
                    self.mark_function_body(FunctionKind::Ordinary);
                    self.open_block();
                    return Out::Consumed;
                }
                self.push_expr(ExprCfg {
                    stop_comma: true,
                    ..ExprCfg::default()
                });
                Out::Retry
            }
            MatchState::AfterBody => {
                if tok.is(b'}') {
                    return Out::Consumed;
                }
                body.state = MatchState::Pattern;
                keep(self, body);
                if tok.is(b',') {
                    Out::Consumed
                } else {
                    Out::Retry
                }
            }
        }
    }
}
