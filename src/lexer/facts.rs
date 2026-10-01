//! Token facts — the lexer layer's one model of where TypeScript statements
//! and expressions end.
//!
//! Automatic semicolon insertion (ECMA-262 §12.10.1), the regular
//! expression/division decision, and every "does this token start a
//! statement" question are the same question in different clothes: which
//! grammar position a token stands in. [`Machine`] answers it once per token
//! stream, while the lexer produces the stream, and records the answer on
//! each [`crate::lexer::Token`] as [`TokenFacts`]. Every later consumer —
//! the parser's statement starts and pipeline heads, flow statement
//! splitting, the program-syntax projection — reads the recorded facts
//! instead of re-deriving them from the tokens around it.
//!
//! The machine is a push-down recognizer for TypeScript's statement and
//! expression skeleton, driven one token at a time so the lexer can ask it
//! whether a `/` starts a regular expression before lexing it. It keeps a
//! stack of grammar frames: statement lists, statements, expressions,
//! bracketed groups (a call's arguments, a control head, a parameter list),
//! object and class bodies, and TypeScript types entered after annotations,
//! assertions, type arguments, heritage clauses, and type aliases. It does
//! not validate: a token no frame expects is absorbed where it stands, so a
//! malformed region never disturbs the facts of the code around it. It
//! knows tt's statement-shaped constructs too (`if let`, let-else, `match`
//! arms, `variant` bodies, `result` blocks), because the streams it reads
//! are tt source.
//!
//! For TypeScript input, the statement spans the machine recognizes are the
//! statement spans SWC parses; the tests check that on a corpus.

mod expressions;
mod statements;
mod types;

#[cfg(test)]
mod tests;

pub(super) use types::{Lookaheads, type_arguments_end};

use crate::ast::Span;
use crate::scanner::{at, ident_end, skip_trivia, starts_identifier};

/// What the lexer knows about one token's grammar position.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TokenFacts(u16);

impl TokenFacts {
    const LINE_BREAK_BEFORE: u16 = 1;
    const ENDS_EXPRESSION: u16 = 1 << 1;
    const ASI_BEFORE: u16 = 1 << 2;
    const STATEMENT_START: u16 = 1 << 3;
    const LABEL: u16 = 1 << 4;
    const MEMBER: u16 = 1 << 5;
    const FUNCTION_BODY: u16 = 1 << 6;
    const GENERATOR_BODY: u16 = 1 << 7;
    const CONSTRUCTOR_BODY: u16 = 1 << 8;
    const TYPE_ARGUMENTS_OPEN: u16 = 1 << 9;
    const TYPE_ARGUMENTS_CLOSE: u16 = 1 << 10;
    const DECLARATION: u16 = 1 << 11;
    const OPERAND_START: u16 = 1 << 12;
    const MODIFIED: u16 = 1 << 13;
    const CLASS_BODY: u16 = 1 << 14;
    const STATIC_BLOCK: u16 = 1 << 15;

    /// A line terminator (ECMA-262 §12.3: LF, CR, U+2028, U+2029), possibly
    /// inside a comment, separates this token from the previous one.
    pub(crate) fn line_break_before(self) -> bool {
        self.0 & Self::LINE_BREAK_BEFORE != 0
    }

    /// This token completes an operand: a value expression, or a type in a
    /// type position, is whole after it.
    pub(crate) fn ends_expression(self) -> bool {
        self.0 & Self::ENDS_EXPRESSION != 0
    }

    /// An automatic semicolon is inserted before this token (§12.10.1): a
    /// line terminator separates it from a statement it cannot continue,
    /// or from a restricted production (`return`, `yield`, `break`,
    /// `continue`, a postfix `++`/`--`).
    pub(crate) fn asi_before(self) -> bool {
        self.0 & Self::ASI_BEFORE != 0
    }

    /// This token begins a statement.
    pub(crate) fn statement_start(self) -> bool {
        self.0 & Self::STATEMENT_START != 0
    }

    /// This identifier is a statement label, where it is declared or where
    /// a `break`/`continue` names it.
    pub(crate) fn label(self) -> bool {
        self.0 & Self::LABEL != 0
    }

    /// This `function` or `class` keyword begins a declaration, which binds
    /// its name in the enclosing scope (ECMA-262 §15.2, §15.7). The same
    /// keyword in an operand position begins an expression, whose name
    /// binds only inside the function or class itself.
    pub(crate) fn declaration(self) -> bool {
        self.0 & Self::DECLARATION != 0
    }

    /// This token begins an operand where an expression expects one: a
    /// value may be written here, as opposed to a statement keyword, a
    /// member name, a binding, or a type.
    pub(crate) fn operand_start(self) -> bool {
        self.0 & Self::OPERAND_START != 0
    }

    /// This token continues a statement that modifiers or decorators began
    /// (`export`, `declare`, `@dec`): what follows them is a declaration.
    pub(crate) fn modified(self) -> bool {
        self.0 & Self::MODIFIED != 0
    }

    /// This token names a member of a class, interface, type literal, or
    /// object literal, where it is declared.
    pub(crate) fn member(self) -> bool {
        self.0 & Self::MEMBER != 0
    }

    /// This `{` opens a body after `=>` or after a parameter list (and its
    /// return type): a function, method, accessor, constructor, or arrow
    /// function body, or a tt `match` arm's block.
    pub(crate) fn function_body(self) -> bool {
        self.0 & Self::FUNCTION_BODY != 0
    }

    /// This function body `{` belongs to a generator.
    pub(crate) fn generator_body(self) -> bool {
        self.0 & Self::GENERATOR_BODY != 0
    }

    /// This function body `{` belongs to a class constructor.
    pub(crate) fn constructor_body(self) -> bool {
        self.0 & Self::CONSTRUCTOR_BODY != 0
    }

    /// This `{` opens a class body (ECMA-262 §15.7, `ClassBody`). What it
    /// holds outside a method's body, a field initializer or a computed
    /// member name, is evaluated by the class definition or by its own
    /// initializer function (§15.7.10, §15.7.14), never by the function the
    /// class is written in, so a `return` there cannot leave that function.
    pub(crate) fn class_body(self) -> bool {
        self.0 & Self::CLASS_BODY != 0
    }

    /// This `{` opens a class static block (ECMA-262 §15.7,
    /// `ClassStaticBlock`). Its statement list is parsed with `[~Return]`,
    /// so it has no function to return from.
    pub(crate) fn static_block(self) -> bool {
        self.0 & Self::STATIC_BLOCK != 0
    }

    /// This `<` opens a list of type arguments or type parameters
    /// (`f<A, B>(x)`, `new Map<K, V>()`, `function g<T>()`, `Array<T>`),
    /// which its matching `>` closes: a bracket pair, not a comparison.
    pub(crate) fn opens_type_arguments(self) -> bool {
        self.0 & Self::TYPE_ARGUMENTS_OPEN != 0
    }

    /// This `>` closes the list a `<` with
    /// [`TokenFacts::opens_type_arguments`] opened.
    pub(crate) fn closes_type_arguments(self) -> bool {
        self.0 & Self::TYPE_ARGUMENTS_CLOSE != 0
    }

    /// The statement or expression before this token ends before it: an
    /// automatic semicolon or a statement start separates them.
    pub(crate) fn boundary_before(self) -> bool {
        self.asi_before() || self.statement_start()
    }

    /// These facts, for a token after which an operand is complete.
    pub(super) fn ending_expression(self) -> Self {
        self.with(Self::ENDS_EXPRESSION)
    }

    fn with(self, flag: u16) -> Self {
        Self(self.0 | flag)
    }
}

impl std::fmt::Debug for TokenFacts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names = [
            (Self::LINE_BREAK_BEFORE, "line-break"),
            (Self::ENDS_EXPRESSION, "ends-expression"),
            (Self::ASI_BEFORE, "asi"),
            (Self::STATEMENT_START, "statement-start"),
            (Self::LABEL, "label"),
            (Self::MEMBER, "member"),
            (Self::FUNCTION_BODY, "function-body"),
            (Self::GENERATOR_BODY, "generator"),
            (Self::CONSTRUCTOR_BODY, "constructor"),
            (Self::TYPE_ARGUMENTS_OPEN, "type-arguments-open"),
            (Self::TYPE_ARGUMENTS_CLOSE, "type-arguments-close"),
            (Self::DECLARATION, "declaration"),
            (Self::OPERAND_START, "operand-start"),
            (Self::MODIFIED, "modified"),
            (Self::CLASS_BODY, "class-body"),
            (Self::STATIC_BLOCK, "static-block"),
        ];
        let set: Vec<&str> = names
            .iter()
            .filter(|(flag, _)| self.0 & flag != 0)
            .map(|(_, name)| *name)
            .collect();
        write!(f, "TokenFacts({})", set.join(" "))
    }
}

/// A token as the machine sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tk {
    Word,
    Str,
    Template,
    Regex,
    Jsx,
    Number,
    Punct(u8),
    Arrow,
    OrOr,
    OptChain,
    Coalesce,
    Pipe,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Tok<'a> {
    pub(super) kind: Tk,
    /// A word's spelling; empty for every other kind, whose rules read the
    /// kind alone.
    pub(super) text: &'a str,
    pub(super) span: Span,
    pub(super) line_break: bool,
}

impl Tok<'_> {
    fn is(&self, byte: u8) -> bool {
        self.kind == Tk::Punct(byte)
    }

    fn word(&self) -> Option<&str> {
        (self.kind == Tk::Word).then_some(self.text)
    }

    fn is_word(&self, word: &str) -> bool {
        self.kind == Tk::Word && self.text == word
    }
}

/// Where a token stream starts in the grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Start {
    /// A statement list: a file, or a body a caller lexes on its own.
    Statements,
    /// One expression: a template interpolation or a JSX expression
    /// container.
    Expression,
}

/// The result of offering a token to the frame on top of the stack.
enum Out {
    /// The token is consumed.
    Consumed,
    /// The frame is finished without consuming the token; offer it to the
    /// frame below.
    Retry,
}

use expressions::{Expr, ExprCfg, Group, GroupKind, Object, Params};
use statements::{ClassMember, Decl, Stmt, SwitchBody, TtIf};
use types::{Type, TypeBody, TypeGroup};

/// Whether `yield` is an operator in a statement list: ECMA-262's `[Yield]`
/// grammar parameter, which a function body sets (§15.5) and a block
/// inherits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Yield {
    /// A block: the enclosing list decides.
    Inherited,
    /// A file, an ordinary function's body, a namespace body, or a class
    /// static block: `yield` is an identifier.
    Identifier,
    /// A generator's body: `yield` begins a `YieldExpression`.
    Operator,
}

/// One grammar position on the machine's stack.
#[derive(Clone, Copy, Debug)]
enum Frame {
    /// A statement list. `closed` lists end at `}`; a list that is itself a
    /// block statement records its start for the statement trace.
    List {
        closed: bool,
        block: Option<usize>,
        yields: Yield,
    },
    /// Exactly one statement: the body of a control statement or a label.
    Slot,
    Stmt(Stmt),
    Decl(Decl),
    TtIf(TtIf),
    Switch(SwitchBody),
    ClassBody(ClassMember),
    Expr(Expr),
    Group(Group),
    Params(Params),
    Object(Object),
    /// A tt `match` body: patterns, guards, and arm bodies.
    Match(statements::MatchBody),
    /// A decorator's expression: a name chain, possibly called.
    Decorator {
        called: bool,
        name: bool,
    },
    Type(Type),
    TypeGroup(TypeGroup),
    TypeBody(TypeBody),
}

/// The next significant position after a token.
#[derive(Clone, Copy, Debug)]
struct Peek {
    at: usize,
    line_break: bool,
}

/// The push-down recognizer. See the module documentation.
pub(super) struct Machine<'s> {
    src: &'s str,
    end: usize,
    stack: Vec<Frame>,
    facts: TokenFacts,
    /// The second byte of a postfix `++`/`--`, which changes nothing.
    skip_next: bool,
    last_end: usize,
    statements: Option<Vec<Span>>,
    /// The frames [`Machine::operand_expected`] puts back after offering a
    /// byte, kept between queries so a query does not allocate.
    probe: Vec<Frame>,
    lookaheads: Lookaheads,
}

impl Frame {
    /// The statement list of a file, or of a region whose expression has
    /// ended.
    fn top_level() -> Self {
        Frame::List {
            closed: false,
            block: None,
            yields: Yield::Identifier,
        }
    }
}

impl<'s> Machine<'s> {
    pub(super) fn new(src: &'s str, end: usize, start: Start, trace: bool) -> Self {
        let mut stack = Vec::with_capacity(32);
        match start {
            Start::Statements => stack.push(Frame::top_level()),
            Start::Expression => stack.push(Frame::Expr(Expr::new(ExprCfg::default()))),
        }
        Machine {
            src,
            end,
            stack,
            facts: TokenFacts::default(),
            skip_next: false,
            last_end: 0,
            statements: trace.then(Vec::new),
            probe: Vec::new(),
            lookaheads: Lookaheads::default(),
        }
    }

    /// Whether an operand may begin at byte `at`, the `/`, `<`, or `.` the
    /// lexer is deciding, which a line terminator precedes when
    /// `line_break`: whether a `/` there starts a regular expression, a `<`
    /// a JSX element, and a `.` a numeric literal — the lexical goal
    /// `InputElementRegExp` of ECMA-262 §12.
    ///
    /// The goal is the grammar's, so the machine's own frames decide it:
    /// the byte is offered, as a punctuator, to a copy of the stack. Frames
    /// the grammar has completed hand it down — a statement or declaration
    /// whose last part is done, one only a keyword (`else`, `catch`,
    /// `finally`, `while`) could continue, one an automatic semicolon ends
    /// before the byte (§12.10.1) — and an operand is expected when an
    /// expression waiting for one receives it.
    pub(super) fn operand_expected(&mut self, at: usize, line_break: bool) -> bool {
        let facts = std::mem::take(&mut self.facts);
        let skip_next = std::mem::take(&mut self.skip_next);
        let last_end = self.last_end;
        let statements = self.statements.take();
        let mut popped = std::mem::take(&mut self.probe);
        popped.clear();
        let mut floor = self.stack.len();
        let expected = self.offer_operand_byte(at, line_break, &mut floor, &mut popped);
        self.stack.truncate(floor);
        self.stack.extend(popped.drain(..).rev());
        self.probe = popped;
        self.facts = facts;
        self.skip_next = skip_next;
        self.last_end = last_end;
        self.statements = statements;
        expected
    }

    /// Offers the byte to the stack as [`Machine::operand_expected`] asks.
    /// The frames the offer pops from below `floor` are kept in `popped`,
    /// top first, so the caller puts the stack back by touching only the
    /// frames the offer reached rather than copying the whole stack.
    fn offer_operand_byte(
        &mut self,
        at: usize,
        line_break: bool,
        floor: &mut usize,
        popped: &mut Vec<Frame>,
    ) -> bool {
        let tok = Tok {
            kind: Tk::Punct(self.src.as_bytes()[at]),
            text: "",
            span: Span {
                start: at,
                end: at + 1,
            },
            line_break,
        };
        for _ in 0..4096 {
            crate::work::tick("operand probe frames");
            let frame = match self.stack.pop() {
                Some(frame) => {
                    if self.stack.len() < *floor {
                        *floor = self.stack.len();
                        popped.push(frame);
                    }
                    frame
                }
                None => Frame::top_level(),
            };
            if let Frame::Expr(expr) = frame
                && expr.operand_expected()
            {
                return true;
            }
            if let Out::Consumed = self.step(frame, &tok) {
                return false;
            }
        }
        false
    }

    /// The lookahead answers this machine's region has found, which a scan
    /// of the same region outside the machine (a JSX tag's type arguments)
    /// shares.
    pub(super) fn lookaheads(&self) -> &Lookaheads {
        &self.lookaheads
    }

    /// Whether `yield` is an operator where the machine stands: the nearest
    /// enclosing statement list that decides it is a generator's body.
    pub(super) fn yield_operator(&self) -> bool {
        self.stack.iter().rev().find_map(|frame| match frame {
            Frame::List { yields, .. } if *yields != Yield::Inherited => Some(*yields),
            _ => None,
        }) == Some(Yield::Operator)
    }

    /// Offers one token; returns its facts.
    pub(super) fn push(&mut self, tok: Tok<'_>) -> TokenFacts {
        self.facts = if tok.line_break {
            TokenFacts::default().with(TokenFacts::LINE_BREAK_BEFORE)
        } else {
            TokenFacts::default()
        };
        if std::mem::take(&mut self.skip_next) {
            self.last_end = tok.span.end;
            return self.facts.with(TokenFacts::ENDS_EXPRESSION);
        }
        let mut guard = 0usize;
        let mut lowest = self.stack.len();
        loop {
            let Some(frame) = self.stack.pop() else {
                self.stack.push(Frame::top_level());
                continue;
            };
            match self.step(frame, &tok) {
                Out::Consumed => break,
                Out::Retry if self.stack.len() < lowest => {
                    lowest = self.stack.len();
                    guard = 0;
                }
                Out::Retry => {
                    guard += 1;
                    debug_assert!(guard < 4096, "the facts machine made no progress");
                    if guard >= 4096 {
                        break;
                    }
                }
            }
        }
        self.last_end = tok.span.end;
        if self.after_operand() {
            self.facts = self.facts.with(TokenFacts::ENDS_EXPRESSION);
        }
        self.facts
    }

    /// Facts for a token inside an atom the machine already consumed (the
    /// digits and letters after a numeric literal's first byte).
    pub(super) fn continuation(&mut self, span: Span) -> TokenFacts {
        self.last_end = span.end;
        TokenFacts::default().with(TokenFacts::ENDS_EXPRESSION)
    }

    /// Ends the stream, completing every open statement.
    pub(super) fn finish(mut self) -> Vec<Span> {
        while let Some(frame) = self.stack.pop() {
            match frame {
                Frame::Stmt(stmt) => self.record(stmt.start(), self.last_end),
                Frame::Decl(decl) => {
                    if let Some(start) = decl.start() {
                        self.record(start, self.last_end);
                    }
                }
                Frame::TtIf(tt) => {
                    if let Some(start) = tt.start {
                        self.record(start, self.last_end);
                    }
                }
                Frame::List {
                    block: Some(start), ..
                } => self.record(start, self.last_end),
                _ => {}
            }
        }
        self.statements.unwrap_or_default()
    }

    fn after_operand(&self) -> bool {
        match self.stack.last() {
            Some(Frame::Expr(expr)) => expr.after_operand(),
            Some(Frame::Type(ty)) => ty.atom,
            _ => false,
        }
    }

    fn step(&mut self, frame: Frame, tok: &Tok<'_>) -> Out {
        match frame {
            Frame::List {
                closed,
                block,
                yields,
            } => self.list(closed, block, yields, tok),
            Frame::Slot => self.slot(tok),
            Frame::Stmt(stmt) => self.stmt(stmt, tok),
            Frame::Decl(decl) => self.decl(decl, tok),
            Frame::TtIf(tt) => self.tt_if(tt, tok),
            Frame::Switch(body) => self.switch_body(body, tok),
            Frame::ClassBody(body) => self.class_body(body, tok),
            Frame::Expr(expr) => self.expr(expr, tok),
            Frame::Group(group) => self.group(group, tok),
            Frame::Params(params) => self.params(params, tok),
            Frame::Object(object) => self.object(object, tok),
            Frame::Match(body) => self.match_body(body, tok),
            Frame::Decorator { called, name } => self.decorator(called, name, tok),
            Frame::Type(ty) => self.ty(ty, tok),
            Frame::TypeGroup(group) => self.type_group(group, tok),
            Frame::TypeBody(body) => self.type_body(body, tok),
        }
    }

    fn push_frame(&mut self, frame: Frame) {
        self.stack.push(frame);
    }

    fn mark(&mut self, flag: u16) {
        self.facts = self.facts.with(flag);
    }

    fn record(&mut self, start: usize, end: usize) {
        if let Some(statements) = &mut self.statements
            && end > start
        {
            statements.push(Span { start, end });
        }
    }

    /// The significant position after byte `from`.
    fn peek(&self, from: usize) -> Peek {
        let (at, line_break) = skip_trivia(self.src.as_bytes(), from, self.end);
        Peek { at, line_break }
    }

    fn byte(&self, at_index: usize) -> Option<u8> {
        at(self.src.as_bytes(), at_index, self.end)
    }

    /// The identifier at a peeked position.
    fn word_at(&self, peek: Peek) -> Option<&'s str> {
        let bytes = self.src.as_bytes();
        starts_identifier(bytes, peek.at, self.end)
            .then(|| &self.src[peek.at..ident_end(bytes, peek.at, self.end)])
    }

    /// The token after `tok`: its first byte, identifier, and whether a
    /// line terminator comes first.
    fn next_after(&self, tok: &Tok<'_>) -> (Option<u8>, Option<&'s str>, bool) {
        let peek = self.peek(tok.span.end);
        (self.byte(peek.at), self.word_at(peek), peek.line_break)
    }

    fn list(&mut self, closed: bool, block: Option<usize>, yields: Yield, tok: &Tok<'_>) -> Out {
        let list = Frame::List {
            closed,
            block,
            yields,
        };
        if tok.is(b'}') {
            if closed {
                if let Some(start) = block {
                    self.record(start, tok.span.end);
                }
                return Out::Consumed;
            }
            self.push_frame(list);
            return Out::Consumed;
        }
        self.push_frame(list);
        self.statement(tok, None)
    }

    fn slot(&mut self, tok: &Tok<'_>) -> Out {
        if tok.is(b'}') || tok.is(b')') {
            return Out::Retry;
        }
        self.statement(tok, None)
    }

    fn decorator(&mut self, called: bool, name: bool, tok: &Tok<'_>) -> Out {
        if called {
            return Out::Retry;
        }
        match tok.kind {
            Tk::Word if !name => {
                self.push_frame(Frame::Decorator { called, name: true });
                Out::Consumed
            }
            Tk::Punct(b'.') if name && !tok.line_break => {
                self.push_frame(Frame::Decorator {
                    called,
                    name: false,
                });
                Out::Consumed
            }
            Tk::Punct(b'#') if !name => {
                self.push_frame(Frame::Decorator { called, name });
                Out::Consumed
            }
            Tk::Punct(b'(') if !tok.line_break || !name => {
                self.push_frame(Frame::Decorator { called: true, name });
                self.open_group(GroupKind::Call);
                Out::Consumed
            }
            Tk::Punct(b'<') if name && !tok.line_break => {
                self.push_frame(Frame::Decorator { called, name });
                self.open_type_group(b'>');
                Out::Consumed
            }
            _ => Out::Retry,
        }
    }
}

/// ECMAScript reserved words and the strict-mode future reserved words: no
/// statement label, binding, or operand can be spelled with one.
pub(super) fn reserved(word: &str) -> bool {
    keyword(word)
        || matches!(
            word,
            "await"
                | "implements"
                | "interface"
                | "let"
                | "package"
                | "private"
                | "protected"
                | "public"
                | "static"
                | "yield"
        )
}

/// The reserved words TypeScript's scanner gives keyword kinds up to
/// `LastReservedWord`: the words its `isIdentifier` rejects in every
/// context. The strict-mode future reserved words, `await`, and `yield` are
/// identifiers to its parser.
pub(super) fn keyword(word: &str) -> bool {
    matches!(
        word,
        "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "new"
            | "null"
            | "return"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
    )
}

/// Keywords that can only begin a statement or continue one, never stand in
/// an operand. An expression that meets one at its top level has ended.
/// `if`, `try`, and `let` are here although tt reads `if let` and `try` in
/// expression position and `let` can name a sloppy-mode variable: none of
/// them continues an expression, so a tt operand never contains one at its
/// top level.
pub(crate) fn statement_only_keyword(word: &str) -> bool {
    matches!(
        word,
        "break"
            | "case"
            | "catch"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "finally"
            | "for"
            | "if"
            | "let"
            | "return"
            | "switch"
            | "throw"
            | "try"
            | "var"
            | "while"
            | "with"
    )
}
