//! TypeScript types: the small type grammar the machine enters after an
//! annotation `:`, `as`/`satisfies`, type arguments and parameters,
//! heritage clauses, and a type alias's `=`.
//!
//! A type ends where the next token cannot continue it. Array suffixes,
//! type arguments, conditional `extends`, and predicates `is` continue a
//! type only on the same line, as in TypeScript's parser; `|`, `&`, and a
//! qualified name's `.` continue it across a line terminator.

use super::{Frame, Machine, Out, Peek, Tk, Tok, TokenFacts};
use std::cell::RefCell;
use std::collections::HashMap;

use crate::scanner::{at, ident_end, scan_string, skip_trivia, starts_identifier};

#[derive(Clone, Copy, Debug)]
pub(super) struct Type {
    /// A complete type atom has been read.
    pub(super) atom: bool,
    /// The atom is a function type's parameter list, which `=>` continues
    /// with the return type. A `(` opens one only where TypeScript's
    /// `isUnambiguouslyStartOfFunctionType` says so; any other `(` opens a
    /// parenthesized type, and an `=>` after it belongs to the enclosing
    /// arrow function.
    parameters: bool,
    /// TypeScript's `isStartOfFunctionTypeOrConstructorType` has committed
    /// to a signature: `new` or a type-parameter list came first, so the
    /// next `(` opens parameters whatever follows it.
    signature: bool,
    /// The atom is `import`, whose `(…)` is part of it.
    import: bool,
    /// Conditional types waiting for `?`, and for `:`.
    pending_question: u8,
    pending_colon: u8,
}

impl Type {
    pub(super) fn new() -> Self {
        Type {
            atom: false,
            parameters: false,
            signature: false,
            import: false,
            pending_question: 0,
            pending_colon: 0,
        }
    }
}

/// A bracketed list of types: type arguments or parameters (`<…>`), a
/// tuple, an indexed access, a mapped type's key, a function type's
/// parameters.
#[derive(Clone, Copy, Debug)]
pub(super) struct TypeGroup {
    closer: u8,
}

impl TypeGroup {
    pub(super) fn new(closer: u8) -> Self {
        TypeGroup { closer }
    }
}

/// An interface body or type literal, positioned at one member.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TypeBody {
    Start,
    AfterName,
    AfterSignature,
    AfterType,
}

impl Machine<'_> {
    pub(super) fn ty(&mut self, mut ty: Type, tok: &Tok<'_>) -> Out {
        if !ty.atom {
            return self.type_atom(ty, tok);
        }
        let same_line = !tok.line_break;
        let parameters = std::mem::take(&mut ty.parameters);
        let import = std::mem::take(&mut ty.import);
        match tok.kind {
            Tk::Punct(b'.' | b'|' | b'&') => {
                ty.atom = false;
                self.push_frame(Frame::Type(ty));
                Out::Consumed
            }
            Tk::Punct(b'[') if same_line => {
                self.push_frame(Frame::Type(ty));
                self.open_type_group(b']');
                Out::Consumed
            }
            Tk::Punct(b'<') if same_line => {
                self.push_frame(Frame::Type(ty));
                self.open_type_group(b'>');
                Out::Consumed
            }
            Tk::Punct(b'(') if import => {
                self.push_frame(Frame::Type(ty));
                self.open_type_group(b')');
                Out::Consumed
            }
            Tk::Word if same_line && tok.text == "extends" => {
                ty.atom = false;
                ty.pending_question += 1;
                self.push_frame(Frame::Type(ty));
                Out::Consumed
            }
            Tk::Word if same_line && tok.text == "is" => {
                ty.atom = false;
                self.push_frame(Frame::Type(ty));
                Out::Consumed
            }
            Tk::Punct(b'?') if ty.pending_question > 0 => {
                ty.pending_question -= 1;
                ty.pending_colon += 1;
                ty.atom = false;
                self.push_frame(Frame::Type(ty));
                Out::Consumed
            }
            Tk::Punct(b':') if ty.pending_colon > 0 => {
                ty.pending_colon -= 1;
                ty.atom = false;
                self.push_frame(Frame::Type(ty));
                Out::Consumed
            }
            Tk::Arrow if parameters => {
                ty.atom = false;
                self.push_frame(Frame::Type(ty));
                Out::Consumed
            }
            _ => Out::Retry,
        }
    }

    fn type_atom(&mut self, mut ty: Type, tok: &Tok<'_>) -> Out {
        match tok.kind {
            Tk::Word => {
                if self.type_operator(tok) {
                    ty.signature |= tok.text == "new";
                } else {
                    ty.atom = true;
                    ty.import = tok.text == "import";
                }
                self.push_frame(Frame::Type(ty));
                Out::Consumed
            }
            Tk::Str | Tk::Number | Tk::Template | Tk::Regex => {
                ty.atom = true;
                self.push_frame(Frame::Type(ty));
                Out::Consumed
            }
            Tk::Punct(b'(') => {
                ty.atom = true;
                ty.parameters =
                    std::mem::take(&mut ty.signature) || self.starts_function_type(tok.span.end);
                self.push_frame(Frame::Type(ty));
                self.open_type_group(b')');
                Out::Consumed
            }
            Tk::Punct(b'[') => {
                ty.atom = true;
                self.push_frame(Frame::Type(ty));
                self.open_type_group(b']');
                Out::Consumed
            }
            Tk::Punct(b'{') => {
                ty.atom = true;
                self.push_frame(Frame::Type(ty));
                self.push_frame(Frame::TypeBody(TypeBody::Start));
                Out::Consumed
            }
            Tk::Punct(b'<') => {
                ty.signature = true;
                self.push_frame(Frame::Type(ty));
                self.open_type_group(b'>');
                Out::Consumed
            }
            Tk::Punct(b'-' | b'+' | b'|' | b'&') => {
                self.push_frame(Frame::Type(ty));
                Out::Consumed
            }
            Tk::Punct(b'*' | b'?') => {
                ty.atom = true;
                self.push_frame(Frame::Type(ty));
                Out::Consumed
            }
            _ => Out::Retry,
        }
    }

    pub(super) fn type_group(&mut self, group: TypeGroup, tok: &Tok<'_>) -> Out {
        match tok.kind {
            Tk::Punct(byte) if byte == group.closer => {
                if byte == b'>' {
                    self.mark(TokenFacts::TYPE_ARGUMENTS_CLOSE);
                }
                Out::Consumed
            }
            Tk::Punct(b')' | b']' | b'}') => Out::Retry,
            Tk::Punct(b',' | b':' | b'=' | b';') => {
                self.push_frame(Frame::TypeGroup(group));
                self.open_type();
                Out::Consumed
            }
            Tk::Word if matches!(tok.text, "in" | "as" | "extends") => {
                self.push_frame(Frame::TypeGroup(group));
                self.open_type();
                Out::Consumed
            }
            Tk::Word
            | Tk::Str
            | Tk::Number
            | Tk::Template
            | Tk::Regex
            | Tk::Punct(b'(' | b'[' | b'{' | b'<' | b'|' | b'&' | b'*') => {
                self.push_frame(Frame::TypeGroup(group));
                self.open_type();
                Out::Retry
            }
            _ => {
                self.push_frame(Frame::TypeGroup(group));
                Out::Consumed
            }
        }
    }

    pub(super) fn type_body(&mut self, state: TypeBody, tok: &Tok<'_>) -> Out {
        let keep = |m: &mut Self, state: TypeBody| m.push_frame(Frame::TypeBody(state));
        match tok.kind {
            Tk::Punct(b'}') => return Out::Consumed,
            Tk::Punct(b')' | b']') => return Out::Retry,
            Tk::Punct(b';' | b',') => {
                keep(self, TypeBody::Start);
                return Out::Consumed;
            }
            _ => {}
        }
        match state {
            TypeBody::Start => match tok.kind {
                Tk::Punct(b'(' | b'<') => {
                    keep(self, TypeBody::AfterName);
                    Out::Retry
                }
                Tk::Punct(b'[') => {
                    keep(self, TypeBody::AfterName);
                    self.open_type_group(b']');
                    Out::Consumed
                }
                Tk::Word => {
                    let (next, next_word, next_break) = self.next_after(tok);
                    let modifier = matches!(tok.text, "readonly" | "get" | "set")
                        && !next_break
                        && (next_word.is_some()
                            || matches!(next, Some(b'[' | b'"' | b'\'' | b'#'))
                            || next.is_some_and(|byte| byte.is_ascii_digit()));
                    if modifier {
                        keep(self, TypeBody::Start);
                    } else {
                        self.mark(super::TokenFacts::MEMBER);
                        keep(self, TypeBody::AfterName);
                    }
                    Out::Consumed
                }
                Tk::Punct(b'-' | b'+') => {
                    keep(self, TypeBody::Start);
                    Out::Consumed
                }
                _ => {
                    keep(self, TypeBody::AfterName);
                    Out::Consumed
                }
            },
            TypeBody::AfterName | TypeBody::AfterSignature => match tok.kind {
                Tk::Punct(b'?' | b'!' | b'-' | b'+') if state == TypeBody::AfterName => {
                    keep(self, TypeBody::AfterName);
                    Out::Consumed
                }
                Tk::Punct(b'(') => {
                    keep(self, TypeBody::AfterSignature);
                    self.open_type_group(b')');
                    Out::Consumed
                }
                Tk::Punct(b'<') => {
                    keep(self, TypeBody::AfterName);
                    self.open_type_group(b'>');
                    Out::Consumed
                }
                Tk::Punct(b':') => {
                    keep(self, TypeBody::AfterType);
                    self.open_type();
                    Out::Consumed
                }
                _ => {
                    keep(self, TypeBody::Start);
                    Out::Retry
                }
            },
            TypeBody::AfterType => {
                keep(self, TypeBody::Start);
                Out::Retry
            }
        }
    }

    /// Whether the word `tok`, where a type begins, is a prefix of the type
    /// after it rather than a type name, as TypeScript's parser decides:
    /// `keyof`, `unique`, `readonly` (`parseTypeOperatorOrHigher`), `infer`
    /// (`parseInferType`), `typeof` (`parseTypeQuery`), and `new` always;
    /// `abstract` only before `new` (`isStartOfFunctionTypeOrConstructorType`);
    /// `asserts` only before an identifier or keyword on the same line
    /// (`nextTokenIsIdentifierOrKeywordOnSameLine`, an assertion
    /// predicate).
    fn type_operator(&self, tok: &Tok<'_>) -> bool {
        match tok.text {
            "keyof" | "unique" | "readonly" | "infer" | "typeof" | "new" => true,
            "abstract" => self.next_after(tok).1 == Some("new"),
            "asserts" => {
                let (_, next_word, next_break) = self.next_after(tok);
                next_word.is_some() && !next_break
            }
            _ => false,
        }
    }

    /// TypeScript's `isUnambiguouslyStartOfFunctionType`, for the `(` in a
    /// type ending at byte `from`: the parenthesis opens a function type's
    /// parameters when it is empty (`()`), starts a rest parameter
    /// (`(...`), or starts a parameter (`skipParameterStart`: modifiers,
    /// then a binding name or pattern) followed by `:`, `,`, `?`, `=`, or
    /// `) =>`. Otherwise it opens a parenthesized type.
    fn starts_function_type(&self, from: usize) -> bool {
        let bytes = self.src.as_bytes();
        let first = self.peek(from).at;
        match self.byte(first) {
            Some(b')') => return true,
            Some(b'.') if bytes[first..self.end].starts_with(b"...") => return true,
            _ => {}
        }
        let Some(name_end) = self.skip_parameter_start(first) else {
            return false;
        };
        let next = self.peek(name_end).at;
        match self.byte(next) {
            Some(b':' | b',' | b'?') => true,
            Some(b'=') => !matches!(self.byte(next + 1), Some(b'=' | b'>')),
            Some(b')') => {
                let arrow = self.peek(next + 1).at;
                bytes[arrow..self.end].starts_with(b"=>")
            }
            _ => false,
        }
    }

    /// TypeScript's `skipParameterStart` from byte `at`: the end of the
    /// parameter's modifiers (`parseModifiers`, where a modifier keyword
    /// counts only when `nextTokenCanFollowModifier` holds) and its binding
    /// identifier, `this`, or binding pattern.
    fn skip_parameter_start(&self, mut at: usize) -> Option<usize> {
        let bytes = self.src.as_bytes();
        while let Some(word) = self.word_at(Peek {
            at,
            line_break: false,
        }) {
            let next = self.peek(at + word.len());
            let follows = match word {
                "const" => self.word_at(next) == Some("enum"),
                "static" | "export" | "default" => can_follow_modifier(bytes, next.at, self.end),
                "abstract" | "accessor" | "async" | "declare" | "in" | "out" | "override"
                | "private" | "protected" | "public" | "readonly" => {
                    !next.line_break && can_follow_modifier(bytes, next.at, self.end)
                }
                _ => false,
            };
            if !follows {
                break;
            }
            at = next.at;
        }
        match self.byte(at)? {
            b'[' | b'{' => binding_pattern_end(bytes, at, self.end, &self.lookaheads),
            _ if self.src[at..].starts_with("this") && ident_end(bytes, at, self.end) == at + 4 => {
                Some(at + 4)
            }
            _ => binding_identifier_end(bytes, at, self.end),
        }
    }

    /// Whether the `<` at byte `open`, after an operand, opens type
    /// arguments (`f<T>(x)`, `new Map<K, V>()`, an instantiation
    /// expression) rather than a relational operator: the bracket closes
    /// over type-shaped text, and the token after it is one TypeScript's
    /// `canFollowTypeArgumentsInExpression` accepts.
    pub(super) fn type_arguments_follow(&self, open: usize) -> bool {
        type_arguments_end(self.src.as_bytes(), open, self.end, &self.lookaheads)
            .is_some_and(|close| self.can_follow_type_arguments(close))
    }

    fn can_follow_type_arguments(&self, from: usize) -> bool {
        let bytes = self.src.as_bytes();
        let (next, line_break) = skip_trivia(bytes, from, self.end);
        let Some(c) = at(bytes, next, self.end) else {
            return true;
        };
        match c {
            b'(' | b'`' => true,
            b'<' | b'>' | b'+' | b'-' => false,
            _ if line_break => true,
            b')' | b']' | b'}' | b';' | b',' | b'.' | b'?' | b':' | b'=' | b'|' | b'&' | b'^'
            | b'*' | b'/' | b'%' => true,
            _ if starts_identifier(bytes, next, self.end) => matches!(
                &self.src[next..ident_end(bytes, next, self.end)],
                "as" | "satisfies" | "in" | "instanceof"
            ),
            _ => false,
        }
    }
}

/// The answers of the lookaheads below, by the byte each one starts at.
///
/// A lookahead from an opener reads the same bytes, in the same way, as a
/// lookahead from any opener nested inside it, so one scan answers every
/// nested start it passes and records those answers here. A later question
/// about a start a scan has passed is then answered without reading again,
/// and the scans that do run read disjoint stretches of the region: the
/// lookaheads of a region read each byte a bounded number of times however
/// deeply its brackets nest or however many of them are left unclosed.
#[derive(Debug, Default)]
pub(in crate::lexer) struct Lookaheads {
    type_arguments: RefCell<HashMap<usize, Option<usize>>>,
    expressions: RefCell<HashMap<usize, Option<usize>>>,
}

/// The small type-grammar skipper: the byte just past the `>` closing the
/// type arguments or parameters opened by the `<` at `open`, when
/// everything up to it is type-shaped text — names, literals, brackets,
/// `,`, `.`, `|`, `&`, `?`, `:`, `=>`, and `;` inside a type literal.
pub(in crate::lexer) fn type_arguments_end(
    bytes: &[u8],
    open: usize,
    end: usize,
    lookaheads: &Lookaheads,
) -> Option<usize> {
    if let Some(&known) = lookaheads.type_arguments.borrow().get(&open) {
        return known;
    }
    let mut stack: Vec<(u8, Option<usize>)> = vec![(b'>', Some(open))];
    let mut answers = lookaheads.type_arguments.borrow_mut();
    let answer = scan_type_arguments(bytes, open + 1, end, &mut stack, &mut answers);
    for (_, start) in stack {
        if let Some(start) = start {
            answers.insert(start, None);
        }
    }
    answer
}

fn scan_type_arguments(
    bytes: &[u8],
    mut i: usize,
    end: usize,
    stack: &mut Vec<(u8, Option<usize>)>,
    answers: &mut HashMap<usize, Option<usize>>,
) -> Option<usize> {
    loop {
        crate::work::tick("type argument lookahead steps");
        i = skip_trivia(bytes, i, end).0;
        let c = at(bytes, i, end)?;
        if starts_identifier(bytes, i, end) {
            i = ident_end(bytes, i, end);
            continue;
        }
        match c {
            b'"' | b'\'' => i = scan_string(bytes, i, end),
            b'`' => i = skip_type_template(bytes, i, end),
            b';' if stack.last().map(|&(closer, _)| closer) == Some(b'}') => i += 1,
            b'0'..=b'9' | b'.' | b',' | b'|' | b'&' | b'?' | b':' | b'-' => i += 1,
            b'=' if at(bytes, i + 1, end) == Some(b'>') => i += 2,
            b'<' | b'(' | b'[' | b'{' => {
                stack.push(match c {
                    b'<' => (b'>', Some(i)),
                    b'(' => (b')', None),
                    b'[' => (b']', None),
                    _ => (b'}', None),
                });
                i += 1;
            }
            b'>' | b')' | b']' | b'}' => {
                if stack.last().map(|&(closer, _)| closer) != Some(c) {
                    return None;
                }
                let (_, start) = stack.pop()?;
                i += 1;
                if let Some(start) = start {
                    answers.insert(start, Some(i));
                }
                if stack.is_empty() {
                    return Some(i);
                }
            }
            _ => return None,
        }
    }
}

/// TypeScript's `canFollowModifier`, at byte `i`: `[`, `{`, `*`, `...`, or
/// a literal property name (an identifier or keyword, a string, a number).
fn can_follow_modifier(bytes: &[u8], i: usize, end: usize) -> bool {
    match at(bytes, i, end) {
        Some(b'[' | b'{' | b'*' | b'"' | b'\'' | b'0'..=b'9') => true,
        Some(b'.') => bytes[i..end].starts_with(b"..."),
        Some(_) => starts_identifier(bytes, i, end),
        None => false,
    }
}

/// The end of the binding identifier at byte `i`: an identifier that is not
/// one of the words TypeScript's `isIdentifier` rejects.
fn binding_identifier_end(bytes: &[u8], i: usize, end: usize) -> Option<usize> {
    if !starts_identifier(bytes, i, end) {
        return None;
    }
    let name_end = ident_end(bytes, i, end);
    let word = std::str::from_utf8(&bytes[i..name_end]).ok()?;
    (!super::keyword(word)).then_some(name_end)
}

/// The end of a binding element at byte `i`: a binding identifier or a
/// nested pattern (TypeScript's `parseIdentifierOrPattern`).
fn binding_element_end(
    bytes: &[u8],
    i: usize,
    end: usize,
    lookaheads: &Lookaheads,
) -> Option<usize> {
    match at(bytes, i, end)? {
        b'[' | b'{' => binding_pattern_end(bytes, i, end, lookaheads),
        _ => binding_identifier_end(bytes, i, end),
    }
}

/// The byte just past the binding pattern opened by the `[` or `{` at
/// `open`, when TypeScript's `parseArrayBindingPattern` or
/// `parseObjectBindingPattern` reads it without an error: elements are
/// binding identifiers or nested patterns, with an optional `...` and
/// initializer; an object pattern's element is a shorthand identifier or a
/// property name, string, number, or computed name followed by `:` and an
/// element.
fn binding_pattern_end(
    bytes: &[u8],
    open: usize,
    end: usize,
    lookaheads: &Lookaheads,
) -> Option<usize> {
    let close = if bytes[open] == b'[' { b']' } else { b'}' };
    let mut i = open + 1;
    loop {
        i = skip_trivia(bytes, i, end).0;
        let c = at(bytes, i, end)?;
        if c == close {
            return Some(i + 1);
        }
        if close == b']' && c == b',' {
            i += 1;
            continue;
        }
        if bytes[i..end].starts_with(b"...") {
            i = skip_trivia(bytes, i + 3, end).0;
            i = binding_element_end(bytes, i, end, lookaheads)?;
        } else if close == b']' {
            i = binding_element_end(bytes, i, end, lookaheads)?;
        } else {
            let shorthand = binding_identifier_end(bytes, i, end);
            i = match c {
                b'"' | b'\'' => scan_string(bytes, i, end),
                b'[' => {
                    expression_end(bytes, i + 1, end, lookaheads)
                        .filter(|&close| at(bytes, close, end) == Some(b']'))?
                        + 1
                }
                b'0'..=b'9' => {
                    let mut j = i;
                    while at(bytes, j, end).is_some_and(|b| b.is_ascii_alphanumeric() || b == b'.')
                    {
                        j += 1;
                    }
                    j
                }
                _ if starts_identifier(bytes, i, end) => ident_end(bytes, i, end),
                _ => return None,
            };
            let colon = skip_trivia(bytes, i, end).0;
            if at(bytes, colon, end) == Some(b':') {
                i = skip_trivia(bytes, colon + 1, end).0;
                i = binding_element_end(bytes, i, end, lookaheads)?;
            } else if shorthand != Some(i) {
                return None;
            }
        }
        i = skip_trivia(bytes, i, end).0;
        if at(bytes, i, end) == Some(b'=') && at(bytes, i + 1, end) != Some(b'=') {
            i = expression_end(bytes, i + 1, end, lookaheads)?;
        }
        match at(bytes, i, end)? {
            b',' => i += 1,
            c if c == close => return Some(i + 1),
            _ => return None,
        }
    }
}

/// The position of the `,`, `)`, `]`, or `}` that ends the expression
/// starting at byte `i`, balancing brackets and skipping strings and
/// templates: an initializer or a computed name inside a binding pattern.
///
/// The starts a binding pattern asks from, the byte after a `[` or an `=`,
/// that the scan passes are answered by the same scan ([`Lookaheads`]).
fn expression_end(bytes: &[u8], i: usize, end: usize, lookaheads: &Lookaheads) -> Option<usize> {
    if let Some(&known) = lookaheads.expressions.borrow().get(&i) {
        return known;
    }
    let mut pending: Vec<(usize, usize)> = vec![(0, i)];
    let mut answers = lookaheads.expressions.borrow_mut();
    let answer = scan_expression(bytes, i, end, &mut pending, &mut answers);
    for (_, start) in pending {
        answers.insert(start, None);
    }
    answer
}

fn scan_expression(
    bytes: &[u8],
    mut i: usize,
    end: usize,
    pending: &mut Vec<(usize, usize)>,
    answers: &mut HashMap<usize, Option<usize>>,
) -> Option<usize> {
    let mut depth = 0usize;
    loop {
        crate::work::tick("expression lookahead steps");
        i = skip_trivia(bytes, i, end).0;
        let c = at(bytes, i, end)?;
        if matches!(c, b',' | b')' | b']' | b'}') {
            while let Some(&(at_depth, start)) = pending.last()
                && at_depth == depth
            {
                pending.pop();
                answers.insert(start, Some(i));
            }
        }
        match c {
            b'"' | b'\'' => i = scan_string(bytes, i, end),
            b'`' => i = skip_type_template(bytes, i, end),
            b'(' | b'[' | b'{' => {
                depth += 1;
                i += 1;
                if c == b'[' {
                    pending.push((depth, i));
                }
            }
            b',' | b')' | b']' | b'}' if depth == 0 => return Some(i),
            b')' | b']' | b'}' => {
                depth -= 1;
                i += 1;
            }
            b'=' => {
                i += 1;
                pending.push((depth, i));
            }
            _ => i += 1,
        }
    }
}

/// Skips a template literal type starting at the backtick at `i`.
fn skip_type_template(bytes: &[u8], mut i: usize, end: usize) -> usize {
    i += 1;
    let mut depth = 0usize;
    while i < end {
        match bytes[i] {
            b'\\' => i += 2,
            b'`' if depth == 0 => return i + 1,
            b'$' if at(bytes, i + 1, end) == Some(b'{') => {
                depth += 1;
                i += 2;
            }
            b'}' if depth > 0 => {
                depth -= 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    end
}
