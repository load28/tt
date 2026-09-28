//! TypeScript types: the small type grammar the machine enters after an
//! annotation `:`, `as`/`satisfies`, type arguments and parameters,
//! heritage clauses, and a type alias's `=`.
//!
//! A type ends where the next token cannot continue it. Array suffixes,
//! type arguments, conditional `extends`, and predicates `is` continue a
//! type only on the same line, as in TypeScript's parser; `|`, `&`, and a
//! qualified name's `.` continue it across a line terminator.

use super::{Frame, Machine, Out, Tk, Tok};
use crate::scanner::{at, ident_end, scan_string, skip_trivia, starts_identifier};

#[derive(Clone, Copy, Debug)]
pub(super) struct Type {
    /// A complete type atom has been read.
    pub(super) atom: bool,
    /// The atom is a parenthesized group, which `=>` turns into a function
    /// type's parameters.
    after_paren: bool,
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
            after_paren: false,
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
        let after_paren = std::mem::take(&mut ty.after_paren);
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
            Tk::Arrow if after_paren => {
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
                match tok.text {
                    "keyof" | "typeof" | "readonly" | "unique" | "infer" | "asserts" | "new"
                    | "abstract" => {}
                    word => {
                        ty.atom = true;
                        ty.import = word == "import";
                    }
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
                ty.after_paren = true;
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
            Tk::Punct(byte) if byte == group.closer => Out::Consumed,
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
            Tk::Punct(b'?' | b'.' | b'-' | b'+' | b'!') => {
                self.push_frame(Frame::TypeGroup(group));
                Out::Consumed
            }
            _ => {
                self.push_frame(Frame::TypeGroup(group));
                self.open_type();
                Out::Retry
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

    /// Whether the `<` at byte `open`, after an operand, opens type
    /// arguments (`f<T>(x)`, `new Map<K, V>()`, an instantiation
    /// expression) rather than a relational operator: the bracket closes
    /// over type-shaped text, and the token after it is one TypeScript's
    /// `canFollowTypeArgumentsInExpression` accepts.
    pub(super) fn type_arguments_follow(&self, open: usize) -> bool {
        type_arguments_end(self.src.as_bytes(), open, self.end)
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

/// The small type-grammar skipper: the byte just past the `>` closing the
/// type arguments or parameters opened by the `<` at `open`, when
/// everything up to it is type-shaped text — names, literals, brackets,
/// `,`, `.`, `|`, `&`, `?`, `:`, `=>`, and `;` inside a type literal.
pub(in crate::lexer) fn type_arguments_end(bytes: &[u8], open: usize, end: usize) -> Option<usize> {
    let mut stack: Vec<u8> = vec![b'>'];
    let mut i = open + 1;
    loop {
        i = skip_trivia(bytes, i, end).0;
        let c = at(bytes, i, end)?;
        if starts_identifier(bytes, i, end) {
            i = ident_end(bytes, i, end);
            continue;
        }
        match c {
            b'"' | b'\'' => i = scan_string(bytes, i, end),
            b'`' => i = skip_type_template(bytes, i, end),
            b';' if stack.last() == Some(&b'}') => i += 1,
            b'0'..=b'9' | b'.' | b',' | b'|' | b'&' | b'?' | b':' | b'-' => i += 1,
            b'=' if at(bytes, i + 1, end) == Some(b'>') => i += 2,
            b'<' | b'(' | b'[' | b'{' => {
                stack.push(match c {
                    b'<' => b'>',
                    b'(' => b')',
                    b'[' => b']',
                    _ => b'}',
                });
                i += 1;
            }
            b'>' | b')' | b']' | b'}' => {
                if stack.pop() != Some(c) {
                    return None;
                }
                i += 1;
                if stack.is_empty() {
                    return Some(i);
                }
            }
            _ => return None,
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
