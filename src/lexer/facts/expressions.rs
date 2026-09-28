//! Expressions, bracketed groups, parameter lists, and object literals.

use super::statements::{FunctionKind, MatchBody, TtIf};
use super::types::{Type, TypeGroup};
use super::{Frame, Machine, Out, Tk, Tok, TokenFacts, statement_only_keyword};

/// What ended the operand an expression is past, which decides the tokens
/// that may continue it (ECMA-262 §13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum After {
    /// A primary or member expression: calls, member access, indexing,
    /// tagged templates, postfix operators, and binary operators continue.
    Primary,
    /// A TypeScript `as`/`satisfies` type: only binary operators continue,
    /// as in TypeScript's `parseBinaryExpressionRest`.
    Assertion,
    /// An arrow function with a block body, which is an
    /// `AssignmentExpression`: nothing but a comma continues it.
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum State {
    /// An operand is expected.
    Operand,
    /// An arrow body is expected: a `{` opens a function body.
    ArrowBody,
    /// An operand has ended.
    After(After),
    /// A property name after `.`.
    Name,
    /// The operation after `?.`.
    OptChain,
}

/// What the frame below an expression wants to take back from it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct ExprCfg {
    /// A `,` separates list items rather than continuing a comma
    /// expression.
    pub(super) stop_comma: bool,
    /// The expression is a statement's own, so a statement keyword at its
    /// top level ends it, and the statement with it.
    pub(super) statement: bool,
    /// A `{` after an operand belongs to the frame below (a class heritage
    /// clause, an `if let` scrutinee).
    pub(super) brace_ends: bool,
    /// A `:` with no pending `?` ends it (a `case` test).
    pub(super) colon_ends: bool,
    /// `<` after an operand always opens type arguments (a heritage
    /// clause's `ExpressionWithTypeArguments`).
    pub(super) heritage: bool,
    /// A `=>` after an operand ends the expression (a `match` guard).
    pub(super) arrow_ends: bool,
}

/// Recognition of a tt `match (…) {` head inside an expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Head {
    None,
    MatchWord,
    MatchCall,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Expr {
    state: State,
    ternary: u16,
    cfg: ExprCfg,
    head: Head,
    /// The operand just ended is a parenthesized group, which a `:` can
    /// follow as an arrow function's return type.
    after_paren: bool,
}

impl Expr {
    pub(super) fn new(cfg: ExprCfg) -> Self {
        Expr {
            state: State::Operand,
            ternary: 0,
            cfg,
            head: Head::None,
            after_paren: false,
        }
    }

    pub(super) fn operand_expected(&self) -> bool {
        matches!(self.state, State::Operand | State::ArrowBody)
    }

    pub(super) fn after_operand(&self) -> bool {
        matches!(self.state, State::After(_))
    }

    fn after(&mut self, after: After) {
        self.state = State::After(after);
        self.after_paren = false;
    }
}

/// The kind of a bracketed list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GroupKind {
    /// A call's arguments.
    Call,
    /// A parenthesized expression or an arrow function's parameters.
    Grouping,
    /// The head of `if`/`while`/`switch`/`with`/`catch`-less control.
    Control,
    /// An array literal or array binding pattern.
    Array,
    /// An element access.
    Index,
    /// A computed property name or an index signature.
    Computed,
}

impl GroupKind {
    fn closer(self) -> u8 {
        match self {
            GroupKind::Call | GroupKind::Grouping | GroupKind::Control => b')',
            GroupKind::Array | GroupKind::Index | GroupKind::Computed => b']',
        }
    }

    fn cfg(self) -> ExprCfg {
        ExprCfg {
            stop_comma: matches!(
                self,
                GroupKind::Call | GroupKind::Grouping | GroupKind::Array
            ),
            ..ExprCfg::default()
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Group {
    pub(super) kind: GroupKind,
}

/// A formal parameter list, positioned at one parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Params {
    Start,
    AfterName,
    AfterType,
    AfterDefault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ObjectKind {
    /// An object literal or object binding pattern.
    Literal,
    /// An `enum` body: members with `=` initializers.
    Enum,
    /// A tt `variant` body: cases with parenthesized fields.
    Variant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ObjectState {
    Key,
    AfterKey,
    AfterParams,
    AfterValue,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Object {
    pub(super) kind: ObjectKind,
    pub(super) state: ObjectState,
    /// The member being read is a generator method (`*name() {}`).
    pub(super) generator: bool,
}

/// Whether the byte after `?` makes it an optional marker (`a?: T`,
/// `(a?) =>`, `a?,`) rather than a conditional operator.
fn optional_marker(next: Option<u8>, after: Option<u8>) -> bool {
    match next {
        Some(b':' | b',' | b')') => true,
        Some(b'=') => after != Some(b'=') && after != Some(b'>'),
        _ => false,
    }
}

impl Machine<'_> {
    pub(super) fn push_expr(&mut self, cfg: ExprCfg) {
        self.push_frame(Frame::Expr(Expr::new(cfg)));
    }

    /// Opens a bracketed list whose `(`/`[` is the current token.
    pub(super) fn open_group(&mut self, kind: GroupKind) {
        self.push_frame(Frame::Group(Group { kind }));
        self.push_expr(kind.cfg());
    }

    pub(super) fn open_params(&mut self) {
        self.push_frame(Frame::Params(Params::Start));
    }

    pub(super) fn open_object(&mut self, kind: ObjectKind) {
        self.push_frame(Frame::Object(Object {
            kind,
            state: ObjectState::Key,
            generator: false,
        }));
    }

    pub(super) fn open_function_body(&mut self) {
        self.push_frame(Frame::List {
            closed: true,
            block: None,
        });
    }

    pub(super) fn open_type(&mut self) {
        self.push_frame(Frame::Type(Type::new()));
    }

    pub(super) fn open_type_group(&mut self, closer: u8) {
        self.push_frame(Frame::TypeGroup(TypeGroup::new(closer)));
        self.open_type();
    }

    pub(super) fn expr(&mut self, mut e: Expr, tok: &Tok<'_>) -> Out {
        match e.state {
            State::Operand | State::ArrowBody => self.expr_operand(e, tok),
            State::After(after) => self.expr_after(e, after, tok),
            State::Name => match tok.kind {
                Tk::Word => {
                    e.after(After::Primary);
                    self.push_frame(Frame::Expr(e));
                    Out::Consumed
                }
                Tk::Punct(b'#') => {
                    self.push_frame(Frame::Expr(e));
                    Out::Consumed
                }
                _ => {
                    e.after(After::Primary);
                    self.push_frame(Frame::Expr(e));
                    Out::Retry
                }
            },
            State::OptChain => match tok.kind {
                Tk::Word => {
                    e.after(After::Primary);
                    self.push_frame(Frame::Expr(e));
                    Out::Consumed
                }
                Tk::Punct(b'#') => {
                    e.state = State::Name;
                    self.push_frame(Frame::Expr(e));
                    Out::Consumed
                }
                Tk::Punct(b'(') => {
                    e.after(After::Primary);
                    self.push_frame(Frame::Expr(e));
                    self.open_group(GroupKind::Call);
                    Out::Consumed
                }
                Tk::Punct(b'[') => {
                    e.after(After::Primary);
                    self.push_frame(Frame::Expr(e));
                    self.open_group(GroupKind::Index);
                    Out::Consumed
                }
                Tk::Punct(b'<') => {
                    e.state = State::OptChain;
                    self.push_frame(Frame::Expr(e));
                    self.open_type_group(b'>');
                    Out::Consumed
                }
                _ => {
                    e.after(After::Primary);
                    self.push_frame(Frame::Expr(e));
                    Out::Retry
                }
            },
        }
    }

    fn atom(&mut self, mut e: Expr) -> Out {
        e.after(After::Primary);
        e.head = Head::None;
        self.push_frame(Frame::Expr(e));
        Out::Consumed
    }

    fn expr_operand(&mut self, mut e: Expr, tok: &Tok<'_>) -> Out {
        let arrow_body = e.state == State::ArrowBody;
        if arrow_body {
            e.state = State::Operand;
        }
        match tok.kind {
            Tk::Word => self.operand_word(e, tok),
            Tk::Str | Tk::Template | Tk::Regex | Tk::Number | Tk::Jsx => self.atom(e),
            Tk::Punct(b'(') => {
                e.after(After::Primary);
                e.after_paren = true;
                e.head = Head::None;
                self.push_frame(Frame::Expr(e));
                self.open_group(GroupKind::Grouping);
                Out::Consumed
            }
            Tk::Punct(b'[') => {
                e.after(After::Primary);
                e.head = Head::None;
                self.push_frame(Frame::Expr(e));
                self.open_group(GroupKind::Array);
                Out::Consumed
            }
            Tk::Punct(b'{') => {
                e.head = Head::None;
                if arrow_body {
                    e.after(After::Closed);
                    self.push_frame(Frame::Expr(e));
                    self.mark_function_body(FunctionKind::Ordinary);
                    self.open_function_body();
                } else {
                    e.after(After::Primary);
                    self.push_frame(Frame::Expr(e));
                    self.open_object(ObjectKind::Literal);
                }
                Out::Consumed
            }
            Tk::Punct(b'<') => {
                self.push_frame(Frame::Expr(e));
                self.open_type_group(b'>');
                Out::Consumed
            }
            Tk::Punct(b'@') => {
                self.push_frame(Frame::Expr(e));
                self.push_frame(Frame::Decorator {
                    called: false,
                    name: false,
                });
                Out::Consumed
            }
            Tk::Punct(b')' | b']' | b'}' | b';' | b',' | b':') => Out::Retry,
            Tk::Arrow => {
                e.state = State::ArrowBody;
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
            Tk::Punct(_) | Tk::OrOr | Tk::Coalesce | Tk::Pipe | Tk::OptChain => {
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
        }
    }

    fn operand_word(&mut self, mut e: Expr, tok: &Tok<'_>) -> Out {
        let (next, next_word, next_break) = self.next_after(tok);
        match tok.text {
            "function" => {
                e.after(After::Primary);
                self.push_frame(Frame::Expr(e));
                self.push_frame(Frame::Decl(super::statements::Decl::function(None)));
                Out::Consumed
            }
            "class" => {
                e.after(After::Primary);
                self.push_frame(Frame::Expr(e));
                self.push_frame(Frame::Decl(super::statements::Decl::class(None)));
                Out::Consumed
            }
            "async"
                if !next_break
                    && (next_word.is_some()
                        && !matches!(
                            next_word,
                            Some("in" | "instanceof" | "as" | "satisfies")
                        )) =>
            {
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
            "new" if next != Some(b'.') => {
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
            "typeof" | "void" | "delete" | "try" => {
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
            "await" | "yield"
                if !(next.is_none()
                    || matches!(next, Some(b')' | b']' | b'}' | b',' | b';' | b':'))
                    || (tok.text == "yield" && next_break)
                    || matches!(next_word, Some("in" | "instanceof" | "of"))) =>
            {
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
            "if" => {
                e.after(After::Primary);
                self.push_frame(Frame::Expr(e));
                self.push_frame(Frame::TtIf(TtIf::new(None)));
                Out::Consumed
            }
            "let" => self.atom(e),
            word if statement_only_keyword(word) && e.cfg.statement => Out::Retry,
            "match" => {
                e.after(After::Primary);
                e.head = Head::MatchWord;
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
            _ => self.atom(e),
        }
    }

    /// Whether `tok`, after a line terminator, continues an expression that
    /// ended in `after`. A token that does not is where an automatic
    /// semicolon goes (§12.10.1); a closer or separator the frames below
    /// take is never one.
    fn continues(&self, e: &Expr, after: After, tok: &Tok<'_>) -> bool {
        let primary = after == After::Primary;
        let binary = after != After::Closed;
        match tok.kind {
            Tk::Punct(b')' | b']' | b'}' | b';' | b',' | b':') => true,
            Tk::Punct(b'{') => false,
            Tk::Punct(b'(' | b'[' | b'.') | Tk::Template | Tk::OptChain => primary,
            Tk::Punct(b'!' | b'~' | b'@' | b'#') => false,
            Tk::Punct(byte @ (b'+' | b'-')) => binary && self.byte(tok.span.end) != Some(byte),
            Tk::Punct(b'=' | b'?') => binary,
            Tk::Punct(_) => binary,
            Tk::Arrow => !e.cfg.arrow_ends,
            Tk::OrOr | Tk::Coalesce | Tk::Pipe => binary,
            Tk::Word => binary && matches!(tok.text, "in" | "instanceof"),
            Tk::Str | Tk::Number | Tk::Regex | Tk::Jsx => false,
        }
    }

    fn expr_after(&mut self, mut e: Expr, after: After, tok: &Tok<'_>) -> Out {
        if tok.line_break && !self.continues(&e, after, tok) {
            return Out::Retry;
        }
        let primary = after == After::Primary;
        let head = std::mem::replace(&mut e.head, Head::None);
        match tok.kind {
            Tk::Word => match tok.text {
                "in" | "instanceof" if after != After::Closed => {
                    e.state = State::Operand;
                    self.push_frame(Frame::Expr(e));
                    Out::Consumed
                }
                "as" | "satisfies" if after != After::Closed => {
                    e.after(After::Assertion);
                    self.push_frame(Frame::Expr(e));
                    self.open_type();
                    Out::Consumed
                }
                _ => Out::Retry,
            },
            Tk::Punct(b'.') if primary => {
                e.state = State::Name;
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
            Tk::OptChain if primary => {
                e.state = State::OptChain;
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
            Tk::Punct(b'(') if primary => {
                e.after(After::Primary);
                if head == Head::MatchWord {
                    e.head = Head::MatchCall;
                }
                self.push_frame(Frame::Expr(e));
                self.open_group(GroupKind::Call);
                Out::Consumed
            }
            Tk::Punct(b'[') if primary => {
                e.after(After::Primary);
                self.push_frame(Frame::Expr(e));
                self.open_group(GroupKind::Index);
                Out::Consumed
            }
            Tk::Template if primary => {
                e.after(After::Primary);
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
            Tk::Punct(b'!') if primary => {
                e.after(After::Primary);
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
            Tk::Punct(byte @ (b'+' | b'-')) => {
                if primary && self.byte(tok.span.end) == Some(byte) {
                    self.skip_next = true;
                    e.after(After::Primary);
                } else {
                    e.state = State::Operand;
                }
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
            Tk::Punct(b'?') => {
                let peek = self.peek(tok.span.end);
                if optional_marker(self.byte(peek.at), self.byte(peek.at + 1)) {
                    e.after(After::Primary);
                } else {
                    e.ternary += 1;
                    e.state = State::Operand;
                }
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
            Tk::Punct(b':') => {
                if e.ternary > 0 && !e.cfg.colon_ends {
                    e.ternary -= 1;
                    e.state = State::Operand;
                    self.push_frame(Frame::Expr(e));
                    Out::Consumed
                } else if e.after_paren && !e.cfg.colon_ends && e.ternary == 0 {
                    e.after(After::Primary);
                    self.push_frame(Frame::Expr(e));
                    self.open_type();
                    Out::Consumed
                } else {
                    Out::Retry
                }
            }
            Tk::Punct(b',') => {
                if e.cfg.stop_comma {
                    return Out::Retry;
                }
                e.state = State::Operand;
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
            Tk::Punct(b';' | b')' | b']' | b'}' | b'@' | b'#') => Out::Retry,
            Tk::Punct(b'{') => {
                if e.cfg.brace_ends || !primary || tok.line_break {
                    return Out::Retry;
                }
                e.after(After::Primary);
                self.push_frame(Frame::Expr(e));
                if head == Head::MatchCall {
                    self.push_frame(Frame::Match(MatchBody::new()));
                } else {
                    self.push_frame(Frame::List {
                        closed: true,
                        block: None,
                    });
                }
                Out::Consumed
            }
            Tk::Punct(b'<')
                if primary && (e.cfg.heritage || self.type_arguments_follow(tok.span.start)) =>
            {
                e.after(After::Primary);
                if head == Head::MatchWord {
                    e.head = Head::MatchWord;
                }
                self.push_frame(Frame::Expr(e));
                self.open_type_group(b'>');
                Out::Consumed
            }
            Tk::Punct(b'<') => {
                self.skip_next = self.byte(tok.span.end) == Some(b'<');
                e.state = State::Operand;
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
            Tk::Arrow => {
                if e.cfg.arrow_ends {
                    return Out::Retry;
                }
                e.state = State::ArrowBody;
                e.after_paren = false;
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
            Tk::Punct(_) | Tk::OrOr | Tk::Coalesce | Tk::Pipe => {
                e.state = State::Operand;
                e.after_paren = false;
                self.push_frame(Frame::Expr(e));
                Out::Consumed
            }
            Tk::Str | Tk::Number | Tk::Regex | Tk::Jsx | Tk::Template | Tk::OptChain => Out::Retry,
        }
    }

    pub(super) fn group(&mut self, group: Group, tok: &Tok<'_>) -> Out {
        let closer = group.kind.closer();
        match tok.kind {
            Tk::Punct(byte) if byte == closer => Out::Consumed,
            Tk::Punct(b')' | b']' | b'}') => Out::Retry,
            Tk::Punct(b',' | b';' | b'=') => {
                self.push_frame(Frame::Group(group));
                self.push_expr(group.kind.cfg());
                Out::Consumed
            }
            Tk::Punct(b':') => {
                self.push_frame(Frame::Group(group));
                if matches!(group.kind, GroupKind::Grouping | GroupKind::Computed) {
                    self.open_type();
                } else {
                    self.push_expr(group.kind.cfg());
                }
                Out::Consumed
            }
            Tk::Punct(b'?') => {
                self.push_frame(Frame::Group(group));
                Out::Consumed
            }
            _ => {
                self.push_frame(Frame::Group(group));
                self.push_expr(group.kind.cfg());
                Out::Retry
            }
        }
    }

    pub(super) fn params(&mut self, state: Params, tok: &Tok<'_>) -> Out {
        let keep = |m: &mut Self, state: Params| m.push_frame(Frame::Params(state));
        match (state, tok.kind) {
            (_, Tk::Punct(b')')) => Out::Consumed,
            (_, Tk::Punct(b']' | b'}')) => Out::Retry,
            (_, Tk::Punct(b',')) => {
                keep(self, Params::Start);
                Out::Consumed
            }
            (Params::Start, Tk::Punct(b'@')) => {
                keep(self, Params::Start);
                self.push_frame(Frame::Decorator {
                    called: false,
                    name: false,
                });
                Out::Consumed
            }
            (Params::Start, Tk::Punct(b'.')) => {
                keep(self, Params::Start);
                Out::Consumed
            }
            (Params::Start, Tk::Word) => {
                let (next, next_word, next_break) = self.next_after(tok);
                let modifier = matches!(
                    tok.text,
                    "public" | "private" | "protected" | "readonly" | "override" | "val"
                ) && !next_break
                    && (next_word.is_some() || matches!(next, Some(b'{' | b'[' | b'.')));
                keep(
                    self,
                    if modifier {
                        Params::Start
                    } else {
                        Params::AfterName
                    },
                );
                Out::Consumed
            }
            (Params::Start, Tk::Punct(b'{')) => {
                keep(self, Params::AfterName);
                self.open_object(ObjectKind::Literal);
                Out::Consumed
            }
            (Params::Start, Tk::Punct(b'[')) => {
                keep(self, Params::AfterName);
                self.open_group(GroupKind::Array);
                Out::Consumed
            }
            (Params::AfterName, Tk::Punct(b'?' | b'!')) => {
                keep(self, Params::AfterName);
                Out::Consumed
            }
            (Params::AfterName, Tk::Punct(b':')) => {
                keep(self, Params::AfterType);
                self.open_type();
                Out::Consumed
            }
            (Params::AfterName | Params::AfterType, Tk::Punct(b'=')) => {
                keep(self, Params::AfterDefault);
                self.push_expr(ExprCfg {
                    stop_comma: true,
                    ..ExprCfg::default()
                });
                Out::Consumed
            }
            _ => {
                keep(self, state);
                Out::Consumed
            }
        }
    }

    pub(super) fn object(&mut self, mut object: Object, tok: &Tok<'_>) -> Out {
        let value_cfg = ExprCfg {
            stop_comma: true,
            ..ExprCfg::default()
        };
        if tok.is(b'}') {
            return Out::Consumed;
        }
        if matches!(tok.kind, Tk::Punct(b')' | b']')) {
            return Out::Retry;
        }
        if tok.is(b',') {
            object.state = ObjectState::Key;
            object.generator = false;
            self.push_frame(Frame::Object(object));
            return Out::Consumed;
        }
        match object.state {
            ObjectState::Key => match tok.kind {
                Tk::Punct(b'.') => {
                    object.state = ObjectState::AfterValue;
                    self.push_frame(Frame::Object(object));
                    self.push_expr(value_cfg);
                    Out::Retry
                }
                Tk::Punct(b'[') => {
                    object.state = ObjectState::AfterKey;
                    self.push_frame(Frame::Object(object));
                    self.open_group(GroupKind::Computed);
                    Out::Consumed
                }
                Tk::Punct(b'*') => {
                    object.generator = true;
                    self.push_frame(Frame::Object(object));
                    Out::Consumed
                }
                Tk::Punct(b'#') => {
                    self.push_frame(Frame::Object(object));
                    Out::Consumed
                }
                Tk::Word => {
                    let (next, next_word, next_break) = self.next_after(tok);
                    let named_next = next_word.is_some()
                        || matches!(next, Some(b'[' | b'*' | b'"' | b'\'' | b'#'))
                        || next.is_some_and(|byte| byte.is_ascii_digit());
                    let modifier = object.kind == ObjectKind::Literal
                        && match tok.text {
                            "get" | "set" => named_next,
                            "async" => named_next && !next_break,
                            _ => false,
                        };
                    if !modifier {
                        object.state = ObjectState::AfterKey;
                        self.mark(TokenFacts::MEMBER);
                    }
                    self.push_frame(Frame::Object(object));
                    Out::Consumed
                }
                Tk::Str | Tk::Number => {
                    object.state = ObjectState::AfterKey;
                    self.mark(TokenFacts::MEMBER);
                    self.push_frame(Frame::Object(object));
                    Out::Consumed
                }
                _ => {
                    self.push_frame(Frame::Object(object));
                    Out::Consumed
                }
            },
            ObjectState::AfterKey => match tok.kind {
                Tk::Punct(b':' | b'=') => {
                    object.state = ObjectState::AfterValue;
                    self.push_frame(Frame::Object(object));
                    self.push_expr(value_cfg);
                    Out::Consumed
                }
                Tk::Punct(b'(') => {
                    object.state = ObjectState::AfterParams;
                    self.push_frame(Frame::Object(object));
                    self.open_params();
                    Out::Consumed
                }
                Tk::Punct(b'<') => {
                    self.push_frame(Frame::Object(object));
                    self.open_type_group(b'>');
                    Out::Consumed
                }
                Tk::Punct(b'?' | b'!') => {
                    self.push_frame(Frame::Object(object));
                    Out::Consumed
                }
                _ => {
                    object.state = ObjectState::Key;
                    self.push_frame(Frame::Object(object));
                    Out::Retry
                }
            },
            ObjectState::AfterParams => match tok.kind {
                Tk::Punct(b':') => {
                    self.push_frame(Frame::Object(object));
                    self.open_type();
                    Out::Consumed
                }
                Tk::Punct(b'{') => {
                    self.mark_function_body(if object.generator {
                        FunctionKind::Generator
                    } else {
                        FunctionKind::Ordinary
                    });
                    object.state = ObjectState::AfterValue;
                    object.generator = false;
                    self.push_frame(Frame::Object(object));
                    self.open_function_body();
                    Out::Consumed
                }
                _ => {
                    object.state = ObjectState::Key;
                    self.push_frame(Frame::Object(object));
                    Out::Retry
                }
            },
            ObjectState::AfterValue => {
                object.state = ObjectState::Key;
                self.push_frame(Frame::Object(object));
                Out::Retry
            }
        }
    }
}
