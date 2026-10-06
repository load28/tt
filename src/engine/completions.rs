//! Completion where the checker has nothing to complete: pattern position.
//!
//! A case tag and a payload field name are written in places that lower to
//! a string literal and a destructuring key, so asking TypeScript "what can
//! go here?" asks about the wrong text — there *is* no text of tt's kind in
//! the output. The completion probe that mends an unfinished `x |> .`
//! cannot help either: splicing a placeholder into a pattern produces a
//! pattern, not a question TypeScript can answer.
//!
//! So tt answers, from the analysis' declaration table — the same table
//! that decides exhaustiveness and resolution, under the same shadowing.
//!
//! **Where a pattern is.** A position is a pattern position only inside a
//! pattern the grammar introduces: a match arm's pattern, an `if let`'s, a
//! let-else's. Where a tt construct parses, its parse says so — the arm
//! pattern spans, the `if let` and let-else alternatives — and everything
//! else inside it (a scrutinee, a guard, an arm body, a bound expression)
//! is an expression, where a `(` after a name is a call. Completion is
//! also asked while a construct is being typed and does not parse yet
//! (`match (s) { Circle(r) => r, Po|` has an arm with no body): the parser
//! claims no construct there, so the parser's own partial queries answer
//! instead — [`crate::parser::pattern_site_at`] for which pattern the text
//! is in, and [`crate::parser::arm_headers`] for the arms already written.
//! This module holds no copy of the arm grammar or the pattern heads
//! (TASK-492). The *declarations* always come from the parse; a
//! declaration elsewhere in the file is complete even while a match is
//! being typed.
//!
//! A comment or a literal is never a pattern position, however its text
//! reads: the lexer's trivia and literal tokens say where they are, before
//! any construct is asked (TASK-462).

use std::path::Path;

use crate::analysis::DeclaredVariant;
use crate::ast::{
    GuardExpr, IfLetElse, IfLetStmt, Pattern, Program, ResultItem, Segment, Span, TagPattern,
    TemplateChunk,
};
use crate::lexer::{Token, TokenKind};
use crate::parser::PatternSite;

use super::documents::Texts;
use super::language::Position;

/// What a completion item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtCompletionKind {
    /// A case tag.
    Case,
    /// A payload field name.
    Field,
    /// A literal the scrutinee's type admits.
    Literal,
    /// The wildcard arm `_`. Offered where an arm may be written, and
    /// nowhere else — `if let _ = x` is not tt syntax.
    Wildcard,
}

/// One thing that can be written at the asked-about position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TtCompletion {
    /// What to insert.
    pub label: String,
    /// Which name space it comes from.
    pub kind: TtCompletionKind,
    /// The declaration, rendered — shown beside the label.
    pub detail: String,
    /// Whether an arm already covers this case (an editor sorts these
    /// last, or dims them). Always false for fields.
    pub covered: bool,
    /// The source range the item replaces, when it is not the word at the
    /// position: the whole string literal a literal pattern is being
    /// written in.
    pub range: Option<super::language::Range>,
}

/// What can be written at `position`, or an empty list when the position is
/// not a pattern position tt owns.
///
/// The answer never includes ordinary TypeScript completions: those are the
/// service's, and a consumer merges the two lists.
///
/// This is the stand-alone question: the files `source` imports are read
/// as saved. A session asks [`super::Workspace::tt_completions_at`], which
/// reads its open documents.
pub fn tt_completions_at(path: &Path, source: &str, position: Position) -> Vec<TtCompletion> {
    completions_at(path, source, position, Texts::Disk)
}

pub(super) fn completions_at(
    path: &Path,
    source: &str,
    position: Position,
    texts: Texts<'_>,
) -> Vec<TtCompletion> {
    pattern_question(path, source, position, texts)
        .map(|question| question.finisher().finish(question.items))
        .unwrap_or_default()
}

pub(super) struct PatternQuestion {
    pub(super) items: Vec<TtCompletion>,
    pub(super) typed: Option<TypedSite>,
    literal: Option<super::language::Range>,
}

impl PatternQuestion {
    pub(super) fn finisher(&self) -> Finisher {
        Finisher {
            literal: self.literal,
        }
    }
}

/// What the position asks of the items once they are gathered.
pub(super) struct Finisher {
    literal: Option<super::language::Range>,
}

impl Finisher {
    /// The items as offered: inside a string literal, only the literals,
    /// each replacing the whole literal written so far.
    pub(super) fn finish(&self, items: Vec<TtCompletion>) -> Vec<TtCompletion> {
        match self.literal {
            None => items,
            Some(range) => items
                .into_iter()
                .filter(|item| item.kind == TtCompletionKind::Literal)
                .map(|item| TtCompletion {
                    range: Some(range),
                    ..item
                })
                .collect(),
        }
    }
}

pub(super) enum TypedSite {
    Arm {
        prefix: Option<(usize, usize)>,
        family: Option<PatternFamily>,
        covered: Vec<String>,
        literals: Vec<crate::ast::LiteralValue>,
        position: Option<usize>,
    },
    Field {
        written: Vec<String>,
        claimed: bool,
    },
}

pub(super) fn pattern_question(
    path: &Path,
    source: &str,
    position: Position,
    texts: Texts<'_>,
) -> Option<PatternQuestion> {
    let offset = super::language::source_byte(source, position);
    let (program, tokens) = crate::parser::lex_and_parse_with_kind(
        source,
        crate::SourceKind::from_path(path).unwrap_or_default(),
    );
    let context = context(source, &program, &tokens, offset)?;
    let declarations = super::language::analyses_for(path, source, texts).declarations;
    let prefix = {
        let next = token_at(&tokens, offset);
        [Some(next), next.checked_sub(1)]
            .into_iter()
            .flatten()
            .find(|&index| index < tokens.len() && is_prefix(&tokens, index, offset))
            .map(|index| (tokens[index].span.start, tokens[index].span.end))
    };
    let mut literal = None;
    let (items, typed) = match context {
        Context::Literal { arms, span } => {
            literal = Some(super::language::span_range(source, span.0, span.1));
            (
                Vec::new(),
                Some(TypedSite::Arm {
                    prefix: Some(span),
                    family: Some(PatternFamily::Literals),
                    covered: arms.covered,
                    literals: arms.literals,
                    position: arms.position,
                }),
            )
        }
        Context::Case { of: Some(arms) } => {
            let mut items = match arms.family {
                Some(PatternFamily::Literals) => Vec::new(),
                Some(PatternFamily::Tags | PatternFamily::Instances) | None => {
                    resolve_all(&declarations, &arms.tags)
                        .flat_map(|declared| cases(declared, &arms.covered))
                        .collect::<Vec<_>>()
                }
            };
            items.push(wildcard());
            let typed = arms.single.then_some(TypedSite::Arm {
                prefix,
                family: arms.family,
                covered: arms.covered,
                literals: arms.literals,
                position: arms.position,
            });
            (items, typed)
        }
        Context::Case { of: None } => (
            declarations
                .iter()
                .flat_map(|declared| cases(declared, &[]))
                .collect(),
            None,
        ),
        Context::Field {
            tag,
            written,
            claimed,
        } => (
            resolve_all(&declarations, std::slice::from_ref(&tag))
                .flat_map(|declared| fields(declared, &tag))
                .filter(|field| !written.contains(&field.label))
                .collect(),
            Some(TypedSite::Field { written, claimed }),
        ),
        Context::Nested { tag, field } => (
            resolve_all(&declarations, std::slice::from_ref(&tag))
                .filter_map(|declared| {
                    declared
                        .constructors
                        .iter()
                        .find(|c| c.tag == tag)
                        .and_then(|c| c.fields.as_deref())
                        .and_then(|fields| fields.iter().find(|f| f.name == field))
                        .and_then(|f| type_variant(&declarations, &f.ty))
                })
                .flat_map(|inner| cases(inner, &[]))
                .collect(),
            None,
        ),
    };
    Some(PatternQuestion {
        items: merge_candidates(items),
        typed,
        literal,
    })
}

pub(super) fn wildcard() -> TtCompletion {
    TtCompletion {
        label: "_".to_string(),
        kind: TtCompletionKind::Wildcard,
        detail: "wildcard arm — every remaining case (must be last)".to_string(),
        covered: false,
        range: None,
    }
}

/// A member access whose name the cursor completes: the cursor ends the
/// name being typed (or stands where it will be) right after a `.` or `?.`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberAccess {
    /// The receiver when it is a name or a path of names (`Result`,
    /// `ns.Shape`), the form a namespace's members are asked through.
    /// `None` for any other expression (`f().`, `xs[0].`, `"abc".`,
    /// `x |> .`).
    pub receiver: Option<String>,
}

/// The member access the cursor at `position` completes, read from the
/// token stream: `None` outside one, in a comment, and in a literal.
pub fn member_access_at(path: &Path, source: &str, position: Position) -> Option<MemberAccess> {
    let offset = super::language::source_byte(source, position);
    let tokens = crate::lexer::lex_with_kind(
        source,
        0,
        source.len(),
        crate::SourceKind::from_path(path).unwrap_or_default(),
    );
    let tokens = innermost_tokens(&tokens, offset);
    let dot = member_dot_at(source, tokens, offset)?;
    // The receiver is a path of names when names and `.`s alone run back
    // from the dot to where the expression starts.
    let mut names = Vec::new();
    let mut after = dot;
    let path = loop {
        let Some(name) = after
            .checked_sub(1)
            .filter(|&name| matches!(tokens[name].kind, TokenKind::Ident))
        else {
            break false;
        };
        names.push(text(source, &tokens[name]));
        match name
            .checked_sub(1)
            .map(|previous| (previous, &tokens[previous].kind))
        {
            Some((_, TokenKind::OptChain)) => break false,
            Some((previous, _)) if is_member_dot(tokens, previous) => after = previous,
            _ => break true,
        }
    };
    names.reverse();
    Some(MemberAccess {
        receiver: path.then(|| names.join(".")),
    })
}

/// A member access in served TypeScript needs a complete receiver before its
/// dot. The tt shorthand `|> .` has none until it has been lowered; TypeScript
/// can return globals there, which must not be mistaken for a member answer.
pub(crate) fn typescript_member_access_at(
    source: &str,
    kind: crate::SourceKind,
    offset: usize,
) -> bool {
    let tokens = crate::lexer::lex_with_kind(source, 0, source.len(), kind);
    let tokens = innermost_tokens(&tokens, offset);
    member_dot_at(source, tokens, offset)
        .and_then(|dot| dot.checked_sub(1))
        .is_some_and(|receiver| tokens[receiver].facts.ends_expression())
}

fn member_dot_at(source: &str, tokens: &[Token], offset: usize) -> Option<usize> {
    if inside_text(source, tokens, offset) {
        return None;
    }
    let mut next = token_at(tokens, offset);
    if next > 0 && is_prefix(tokens, next - 1, offset) {
        next -= 1;
    }
    next.checked_sub(1)
        .filter(|&dot| is_member_dot(tokens, dot))
}

/// A tt keyword whose construct can be written at a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtKeyword {
    /// `variant Name { … }`, a declaration.
    Variant,
    /// `match (…) { … }`, an expression.
    Match,
    /// `try expression;`, the statement form.
    Try,
    /// `flow |> …`, an expression.
    Flow,
    /// `result { … }`, an expression.
    Result,
    /// `const Tag(…) = expression else { … };`, a declaration.
    LetElse,
}

impl TtKeyword {
    /// The label a completion item shows.
    pub fn label(self) -> &'static str {
        match self {
            TtKeyword::Variant => "variant",
            TtKeyword::Match => "match",
            TtKeyword::Try => "try",
            TtKeyword::Flow => "flow",
            TtKeyword::Result => "result",
            TtKeyword::LetElse => "let-else",
        }
    }

    /// Where TypeScript ranks a keyword in a completion list
    /// (`SortText.GlobalsOrKeywords`): after the names in scope, among its
    /// own keywords.
    pub fn sort_text(self) -> &'static str {
        "15"
    }

    /// Whether the construct can begin where a word with `facts` stands: a
    /// declaration where a statement begins or after `export`, another
    /// statement where a statement begins, an expression where a statement
    /// or an operand begins.
    fn fits(self, facts: crate::lexer::TokenFacts) -> bool {
        match self {
            TtKeyword::Variant => facts.statement_start() || facts.modified(),
            TtKeyword::Try | TtKeyword::LetElse => facts.statement_start(),
            TtKeyword::Match | TtKeyword::Flow | TtKeyword::Result => {
                facts.statement_start() || (facts.operand_start() && !facts.modified())
            }
        }
    }

    const ALL: [TtKeyword; 6] = [
        TtKeyword::Variant,
        TtKeyword::Match,
        TtKeyword::Try,
        TtKeyword::Flow,
        TtKeyword::Result,
        TtKeyword::LetElse,
    ];
}

/// The tt keywords whose construct can be written at `position`: a
/// statement's where a statement begins, an expression's where an operand
/// may begin — the grammar position the lexer's facts record for the word
/// being typed there. Nothing in a comment or a literal, at a member
/// access, in a pattern, or where TypeScript expects a name of its own (a
/// property, a JSX attribute, an import specifier, a type).
pub fn tt_keywords_at(path: &Path, source: &str, position: Position) -> Vec<TtKeyword> {
    let kind = crate::SourceKind::from_path(path).unwrap_or_default();
    let offset = super::language::source_byte(source, position);
    let (program, tokens) = crate::parser::lex_and_parse_with_kind(source, kind);
    if member_access_at(path, source, position).is_some()
        || context(source, &program, &tokens, offset).is_some()
    {
        return Vec::new();
    }
    let Some(facts) = word_facts(source, kind, offset) else {
        return Vec::new();
    };
    TtKeyword::ALL
        .into_iter()
        .filter(|keyword| keyword.fits(facts))
        .collect()
}

/// The facts of the word the cursor at `offset` is typing, or of a word
/// written there when none is: the grammar position a name at the cursor
/// stands in. `None` in a comment or a literal.
fn word_facts(
    source: &str,
    kind: crate::SourceKind,
    offset: usize,
) -> Option<crate::lexer::TokenFacts> {
    let tokens = crate::lexer::lex_with_kind(source, 0, source.len(), kind);
    let tokens = innermost_tokens(&tokens, offset);
    if inside_text(source, tokens, offset) {
        return None;
    }
    let next = token_at(tokens, offset);
    if next > 0 && is_prefix(tokens, next - 1, offset) {
        return Some(tokens[next - 1].facts);
    }
    let probe = super::language::PROBE_NAME;
    let spliced = format!("{}{probe}{}", &source[..offset], &source[offset..]);
    let tokens = crate::lexer::lex_with_kind(&spliced, 0, spliced.len(), kind);
    let tokens = innermost_tokens(&tokens, offset);
    tokens
        .get(token_at(tokens, offset))
        .filter(|token| token.span.start == offset && matches!(token.kind, TokenKind::Ident))
        .map(|token| token.facts)
}

/// The tokens of the innermost template interpolation around `offset`, or
/// `tokens` when it is in none.
fn innermost_tokens(tokens: &[Token], offset: usize) -> &[Token] {
    crate::stack::grow(|| innermost_tokens_grown(tokens, offset))
}

fn innermost_tokens_grown(tokens: &[Token], offset: usize) -> &[Token] {
    for token in tokens {
        if let TokenKind::Template(parts) = &token.kind {
            for part in parts.iter() {
                if let crate::lexer::TplPart::Interp { span, tokens } = part
                    && span.start <= offset
                    && offset <= span.end
                {
                    return innermost_tokens(tokens, offset);
                }
            }
        }
    }
    tokens
}

/// Whether the token at `index` is a member-access `.` or `?.`, not one of
/// the dots of a spread's `...`.
fn is_member_dot(tokens: &[Token], index: usize) -> bool {
    match tokens[index].kind {
        TokenKind::OptChain => true,
        TokenKind::Punct(b'.') => {
            let touching_dot = |token: &Token| {
                matches!(token.kind, TokenKind::Punct(b'.'))
                    && (token.span.end == tokens[index].span.start
                        || token.span.start == tokens[index].span.end)
            };
            !(index
                .checked_sub(1)
                .is_some_and(|previous| touching_dot(&tokens[previous]))
                || tokens.get(index + 1).is_some_and(touching_dot))
        }
        _ => false,
    }
}

/// Ambiguous declarations can share a tag or field. Present one insertion
/// candidate while preserving each possible declaration in its detail.
fn merge_candidates(items: Vec<TtCompletion>) -> Vec<TtCompletion> {
    let mut merged: Vec<TtCompletion> = Vec::new();
    for item in items {
        if let Some(existing) = merged
            .iter_mut()
            .find(|other| other.label == item.label && other.kind == item.kind)
        {
            if !existing.detail.lines().any(|line| line == item.detail) {
                existing.detail.push('\n');
                existing.detail.push_str(&item.detail);
            }
        } else {
            merged.push(item);
        }
    }
    merged
}

/// The pattern position the cursor is in.
#[derive(Debug, PartialEq)]
enum Context {
    /// A tag is expected. `of` carries the tags already written in the same
    /// match, which is what says *which* variant — `None` when the position
    /// has no such evidence (an `if let`).
    Case { of: Option<ArmTags> },
    /// A payload field name of `tag` is expected; `written` are the fields
    /// the same payload already binds, which a pattern may not repeat.
    /// `claimed` says a parsed construct holds the payload, so its
    /// projection destructures the case and TypeScript can be asked for
    /// the case's properties there.
    Field {
        tag: String,
        written: Vec<String>,
        claimed: bool,
    },
    /// A literal pattern is being written in the string literal at bytes
    /// `span`, at the top level of an arm of the match whose finished arms
    /// are `arms`.
    Literal { arms: ArmTags, span: (usize, usize) },
    /// A nested pattern's tag is expected, in `tag`'s field `field`.
    Nested { tag: String, field: String },
}

fn context(source: &str, program: &Program, tokens: &[Token], offset: usize) -> Option<Context> {
    if let Some(literal) = string_at(source, tokens, offset) {
        return literal_context(source, program, tokens, literal);
    }
    if inside_text(source, tokens, offset) {
        return None;
    }
    // The identifier being typed is not context — step over it.
    let cursor = tokens
        .iter()
        .position(|t| t.span.start >= offset)
        .unwrap_or(tokens.len());
    // The identifier under the cursor is the prefix being typed: it is
    // neither context nor an arm already written.
    let prefix = if cursor < tokens.len() && is_prefix(tokens, cursor, offset) {
        Some(cursor)
    } else if cursor > 0 && is_prefix(tokens, cursor - 1, offset) {
        Some(cursor - 1)
    } else {
        None
    };
    let before = prefix.unwrap_or(cursor);
    if before == 0 || matches!(tokens[before - 1].kind, TokenKind::Arrow) {
        return None;
    }
    let parsed = parsed_at(program, offset);
    let site = match parsed {
        Some(Parsed::Arm { open, from }) => PatternSite::Arm {
            open: token_at(tokens, open),
            start: from.map_or(before, |from| token_at(tokens, from)),
        },
        Some(Parsed::Single { from }) => PatternSite::Single {
            start: token_at(tokens, from),
        },
        Some(Parsed::Expression) => return None,
        None => crate::parser::pattern_site_at(source, tokens, before)?,
    };
    site_context(source, tokens, site, before, prefix, parsed.is_some())
}

/// The string literal token the cursor is inside: after its opening quote
/// and before its closing one, or at the end of one not closed yet.
fn string_at(source: &str, tokens: &[Token], offset: usize) -> Option<usize> {
    let index = token_at(tokens, offset).checked_sub(1)?;
    let token = &tokens[index];
    if !matches!(token.kind, TokenKind::Str) || offset <= token.span.start {
        return None;
    }
    let bytes = &source.as_bytes()[token.span.start..token.span.end];
    let closed = bytes.len() >= 2 && bytes[bytes.len() - 1] == bytes[0];
    (offset < token.span.end || (offset == token.span.end && !closed)).then_some(index)
}

/// A string literal the cursor is in is a literal pattern being written
/// when it starts an alternative at the top level of a match arm's
/// pattern, the place a finished literal arm's pattern would be.
fn literal_context(
    source: &str,
    program: &Program,
    tokens: &[Token],
    index: usize,
) -> Option<Context> {
    let token = &tokens[index];
    let (open, start) = match parsed_at(program, token.span.start) {
        Some(Parsed::Arm { open, from }) => (
            token_at(tokens, open),
            from.map_or(index, |from| token_at(tokens, from)),
        ),
        Some(_) => return None,
        None => match crate::parser::pattern_site_at(source, tokens, index)? {
            PatternSite::Arm { open, start } => (open, start),
            PatternSite::Single { .. } => return None,
        },
    };
    if index < start
        || innermost_paren(tokens, start, index)?.is_some()
        || (index != start && !matches!(tokens[index - 1].kind, TokenKind::Punct(b'|')))
    {
        return None;
    }
    Some(Context::Literal {
        arms: arm_tags(source, tokens, open, None),
        span: (token.span.start, token.span.end),
    })
}

/// What the pattern grammar expects at `before`, inside `site`'s pattern.
fn site_context(
    source: &str,
    tokens: &[Token],
    site: PatternSite,
    before: usize,
    prefix: Option<usize>,
    claimed: bool,
) -> Option<Context> {
    let (PatternSite::Arm { start, .. } | PatternSite::Single { start }) = site;
    if before < start {
        return None;
    }
    let Some(open) = innermost_paren(tokens, start, before)? else {
        // The pattern's top level: a tag starts the pattern or follows `|`.
        if before != start && !matches!(tokens[before - 1].kind, TokenKind::Punct(b'|')) {
            return None;
        }
        return Some(match site {
            PatternSite::Arm { open, .. } => Context::Case {
                of: Some(arm_tags(source, tokens, open, prefix)),
            },
            PatternSite::Single { .. } => Context::Case { of: None },
        });
    };
    if open > start && matches!(tokens[open - 1].kind, TokenKind::Ident) {
        let tag = text(source, &tokens[open - 1]).to_string();
        // `field:` right before the cursor means the nested pattern's tag.
        if before >= open + 3
            && matches!(tokens[before - 1].kind, TokenKind::Punct(b':'))
            && matches!(tokens[before - 2].kind, TokenKind::Ident)
        {
            return Some(Context::Nested {
                tag,
                field: text(source, &tokens[before - 2]).to_string(),
            });
        }
        // Otherwise a field name: at the start of the list or after a comma.
        if matches!(tokens[before - 1].kind, TokenKind::Punct(b'(' | b',')) {
            return Some(Context::Field {
                tag,
                written: written_fields(source, tokens, open, prefix),
                claimed,
            });
        }
        return None;
    }
    // A tuple pattern's parens open the arm; each slot is a case position.
    if let PatternSite::Arm { open: body, .. } = site
        && open == start
        && matches!(
            tokens[before - 1].kind,
            TokenKind::Punct(b'(' | b',' | b'|')
        )
    {
        let mut position = 0;
        let mut depth = 0usize;
        for token in &tokens[open + 1..before] {
            if token.opens_bracket() {
                depth += 1;
            } else if token.closes_bracket() {
                depth = depth.saturating_sub(1);
            } else if depth == 0 && matches!(token.kind, TokenKind::Punct(b',')) {
                position += 1;
            }
        }
        return Some(Context::Case {
            of: Some(headers_tags(
                tokens,
                crate::parser::tuple_arm_headers(source, tokens, body, position),
                prefix,
                Some(position),
            )),
        });
    }
    None
}

/// The field names the payload list opened at `open` already writes: each
/// name at the list's own level that starts an entry, except the one being
/// typed. The list ends at its `)`, or — mid-edit, unclosed — at the arm's
/// `=>`.
fn written_fields(
    source: &str,
    tokens: &[Token],
    open: usize,
    prefix: Option<usize>,
) -> Vec<String> {
    let mut written = Vec::new();
    let mut depth = 0usize;
    for index in open + 1..tokens.len() {
        let token = &tokens[index];
        if token.opens_bracket() {
            depth += 1;
        } else if token.closes_bracket() {
            if depth == 0 {
                break;
            }
            depth -= 1;
        } else if depth == 0 && matches!(token.kind, TokenKind::Arrow) {
            break;
        } else if depth == 0
            && matches!(token.kind, TokenKind::Ident)
            && Some(index) != prefix
            && (index == open + 1 || matches!(tokens[index - 1].kind, TokenKind::Punct(b',')))
        {
            written.push(text(source, token).to_string());
        }
    }
    written
}

/// Whether `offset` is inside a comment or inside a literal token — a
/// string, template, regex, or JSX text. That is prose or data, not code:
/// no tt name is written there, whatever the words in it look like
/// (TASK-462).
fn inside_text(source: &str, tokens: &[Token], offset: usize) -> bool {
    let next = token_at(tokens, offset);
    if let Some(previous) = next.checked_sub(1)
        && offset < tokens[previous].span.end
    {
        return !matches!(tokens[previous].kind, TokenKind::Ident);
    }
    let from = next
        .checked_sub(1)
        .map_or(0, |previous| tokens[previous].span.end);
    let to = tokens
        .get(next)
        .map_or(source.len(), |token| token.span.start);
    crate::lexer::comment_at(source, from, to, offset)
}

/// Whether the token at `index` is the identifier the cursor sits in — the
/// prefix being typed rather than context.
fn is_prefix(tokens: &[Token], index: usize, offset: usize) -> bool {
    matches!(tokens[index].kind, TokenKind::Ident)
        && tokens[index].span.start <= offset
        && offset <= tokens[index].span.end
}

/// The index of the first token at or after byte `at`.
fn token_at(tokens: &[Token], at: usize) -> usize {
    tokens.partition_point(|token| token.span.start < at)
}

/// The innermost `(` a pattern's tokens `start..before` leave open:
/// `Some(None)` at the pattern's top level, `None` when a `[` or `{` is
/// innermost — no pattern is written inside those.
fn innermost_paren(tokens: &[Token], start: usize, before: usize) -> Option<Option<usize>> {
    let mut open = Vec::new();
    for (index, token) in tokens.iter().enumerate().take(before).skip(start) {
        match token.kind {
            _ if token.opens_bracket() => open.push(index),
            _ if token.closes_bracket() => {
                open.pop();
            }
            _ => {}
        }
    }
    match open.last() {
        None => Some(None),
        Some(&index) if matches!(tokens[index].kind, TokenKind::Punct(b'(')) => Some(Some(index)),
        Some(_) => None,
    }
}

/// What a parsed tt construct says about a position inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Parsed {
    /// In a match body whose `{` is at byte `open`: inside the arm pattern
    /// starting at byte `from`, or, when `from` is `None`, in the slot
    /// before an arm or after the last separator.
    Arm { open: usize, from: Option<usize> },
    /// Inside the pattern of an `if let` or a let-else starting at byte
    /// `from`.
    Single { from: usize },
    /// Inside an expression or statement part of a construct — a
    /// scrutinee, a guard, an arm body, a bound expression, a block — with
    /// no nested construct claiming the position.
    Expression,
}

/// The answer of the innermost parsed tt construct containing `offset`, or
/// `None` when no construct claims it.
fn parsed_at(program: &Program, offset: usize) -> Option<Parsed> {
    crate::stack::grow(|| parsed_at_grown(program, offset))
}

fn parsed_at_grown(program: &Program, offset: usize) -> Option<Parsed> {
    program
        .segments
        .iter()
        .find_map(|segment| segment_at(segment, offset))
}

fn segment_at(segment: &Segment, offset: usize) -> Option<Parsed> {
    crate::stack::grow(|| segment_at_grown(segment, offset))
}

fn segment_at_grown(segment: &Segment, offset: usize) -> Option<Parsed> {
    match segment {
        Segment::Match(expr) => {
            if offset < expr.keyword_off || offset > expr.body_close {
                return None;
            }
            if offset <= expr.body_open {
                return Some(parsed_at(&expr.scrutinee, offset).unwrap_or(Parsed::Expression));
            }
            let arms = expr.arms.iter().map(|arm| {
                (
                    arm.pattern_span,
                    arm.guard.as_ref(),
                    &arm.body,
                    arm.body_span,
                    arm.block,
                )
            });
            Some(arms_at(expr.body_open, arms, offset))
        }
        Segment::TupleMatch(expr) => {
            if offset < expr.keyword_off || offset > expr.body_close {
                return None;
            }
            if offset <= expr.body_open {
                return Some(
                    expr.scrutinees
                        .iter()
                        .find_map(|(_, scrutinee)| parsed_at(scrutinee, offset))
                        .unwrap_or(Parsed::Expression),
                );
            }
            let arms = expr.arms.iter().map(|arm| {
                (
                    arm.pattern_span,
                    arm.guard.as_ref(),
                    &arm.body,
                    arm.body_span,
                    arm.block,
                )
            });
            Some(arms_at(expr.body_open, arms, offset))
        }
        Segment::IfLet(stmt) => if_let_at(stmt, offset),
        Segment::LetElse(stmt) => {
            if offset < stmt.owner_span.start || offset >= stmt.owner_span.end {
                return None;
            }
            Some(
                single_at(&stmt.alternatives, offset)
                    .or_else(|| parsed_at(&stmt.expr, offset))
                    .or_else(|| parsed_at(&stmt.else_body, offset))
                    .unwrap_or(Parsed::Expression),
            )
        }
        Segment::Try(stmt) => parsed_at(&stmt.expr, offset),
        Segment::TryExpr(expr) => parsed_at(&expr.expr, offset),
        Segment::Pipe(pipe) => pipe
            .head
            .as_ref()
            .and_then(|head| parsed_at(head, offset))
            .or_else(|| {
                pipe.steps
                    .iter()
                    .find_map(|step| parsed_at(&step.body, offset))
            }),
        Segment::ResultBlock(block) => block
            .items
            .iter()
            .find_map(|item| {
                let ResultItem::Stmts(stmts) = item;
                parsed_at(stmts, offset)
            })
            .or_else(|| {
                block
                    .value
                    .as_ref()
                    .and_then(|value| parsed_at(value, offset))
            }),
        Segment::Template(template) => template.chunks.iter().find_map(|chunk| match chunk {
            TemplateChunk::Interp(interp) => parsed_at(interp, offset),
            TemplateChunk::Raw(_) => None,
        }),
        Segment::Verbatim(_)
        | Segment::Variant(_)
        | Segment::TtImport(_)
        | Segment::ValModifier(_) => None,
    }
}

/// A position inside a parsed match body: in an arm's pattern, in an arm's
/// guard or body, or in a slot between arms.
fn arms_at<'a>(
    open: usize,
    arms: impl Iterator<Item = (Span, Option<&'a GuardExpr>, &'a Program, Span, bool)>,
    offset: usize,
) -> Parsed {
    for (pattern, guard, body, body_span, block) in arms {
        let end = if block {
            body_span.end + 1
        } else {
            body_span.end
        };
        if offset < pattern.start || offset > end {
            continue;
        }
        if offset <= pattern.end {
            return Parsed::Arm {
                open,
                from: Some(pattern.start),
            };
        }
        return guard
            .and_then(|guard| parsed_at(&guard.expr, offset))
            .or_else(|| parsed_at(body, offset))
            .unwrap_or(Parsed::Expression);
    }
    Parsed::Arm { open, from: None }
}

fn if_let_at(stmt: &IfLetStmt, offset: usize) -> Option<Parsed> {
    crate::stack::grow(|| if_let_at_grown(stmt, offset))
}

fn if_let_at_grown(stmt: &IfLetStmt, offset: usize) -> Option<Parsed> {
    if offset < stmt.owner_span.start || offset >= stmt.owner_span.end {
        return None;
    }
    Some(
        single_at(&stmt.alternatives, offset)
            .or_else(|| parsed_at(&stmt.expr, offset))
            .or_else(|| parsed_at(&stmt.body, offset))
            .or_else(|| match &stmt.else_part {
                Some(IfLetElse::Block(block)) => parsed_at(block, offset),
                Some(IfLetElse::IfLet(inner)) => if_let_at(inner, offset),
                None => None,
            })
            .unwrap_or(Parsed::Expression),
    )
}

fn single_at(alternatives: &[TagPattern], offset: usize) -> Option<Parsed> {
    let (first, last) = (alternatives.first()?, alternatives.last()?);
    (first.tag_off <= offset && offset <= last.end).then_some(Parsed::Single {
        from: first.tag_off,
    })
}

/// Completed arm headers provide variant evidence. An unfinished sibling and
/// a wildcard do not identify a variant; expression-body identifiers and pipes
/// are outside the pattern grammar and must never constrain its candidates.
#[derive(Debug, PartialEq)]
pub(super) struct ArmTags {
    tags: Vec<String>,
    covered: Vec<String>,
    family: Option<PatternFamily>,
    literals: Vec<crate::ast::LiteralValue>,
    single: bool,
    position: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PatternFamily {
    Tags,
    Literals,
    Instances,
}

/// The tags the finished arms of the match body at `open` name, as the
/// parser reads their patterns. The tag being typed at `prefix` is not
/// evidence yet. A guarded or nested alternative names a tag without
/// covering it.
fn arm_tags(source: &str, tokens: &[Token], open: usize, prefix: Option<usize>) -> ArmTags {
    headers_tags(
        tokens,
        crate::parser::arm_headers(source, tokens, open),
        prefix,
        None,
    )
}

fn headers_tags(
    tokens: &[Token],
    headers: Vec<crate::parser::ArmHeader>,
    prefix: Option<usize>,
    position: Option<usize>,
) -> ArmTags {
    let prefix = prefix.map(|index| tokens[index].span.start);
    let mut tags = Vec::new();
    let mut covered: Vec<String> = Vec::new();
    let mut family = None;
    let mut literals = Vec::new();
    for header in headers {
        let alternatives = match header.pattern {
            Some(Pattern::Tags(alternatives)) => alternatives,
            Some(Pattern::Literals(alternatives)) => {
                family = Some(PatternFamily::Literals);
                if !header.guarded {
                    literals.extend(alternatives.into_iter().map(|literal| literal.value));
                }
                continue;
            }
            Some(Pattern::Instances(_)) => {
                family.get_or_insert(PatternFamily::Instances);
                continue;
            }
            Some(Pattern::Wildcard) | None => continue,
        };
        if family != Some(PatternFamily::Literals) {
            family = Some(PatternFamily::Tags);
        }
        let nested = alternatives
            .iter()
            .flat_map(|alternative| alternative.bindings.iter().flatten())
            .any(|binding| binding.nested.is_some());
        for alternative in alternatives {
            if prefix == Some(alternative.tag_off) {
                continue;
            }
            if !header.guarded && !nested && !covered.contains(&alternative.tag) {
                covered.push(alternative.tag.clone());
            }
            if !tags.contains(&alternative.tag) {
                tags.push(alternative.tag);
            }
        }
    }
    ArmTags {
        tags,
        covered,
        family,
        literals,
        single: true,
        position,
    }
}

/// Keep every declaration consistent with the known tags. With no evidence,
/// choosing the first table entry would arbitrarily hide other visible cases.
fn resolve_all<'a>(
    declarations: &'a [DeclaredVariant],
    tags: &'a [String],
) -> impl Iterator<Item = &'a DeclaredVariant> {
    declarations.iter().filter(|declared| {
        tags.iter()
            .all(|tag| declared.constructors.iter().any(|c| c.tag == *tag))
    })
}

/// The variant a declared field type names, when it names one plainly.
fn type_variant<'a>(declarations: &'a [DeclaredVariant], ty: &str) -> Option<&'a DeclaredVariant> {
    let trimmed = ty.trim();
    let base: String = trimmed
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '$' | '.'))
        .collect();
    declarations
        .iter()
        .find(|d| d.name == base || d.type_names.contains(&base))
}

fn cases(declared: &DeclaredVariant, covered: &[String]) -> Vec<TtCompletion> {
    declared
        .constructors
        .iter()
        .map(|constructor| TtCompletion {
            label: constructor.tag.clone(),
            kind: TtCompletionKind::Case,
            detail: super::names::case_signature(&declared.name, constructor),
            covered: covered.contains(&constructor.tag),
            range: None,
        })
        .collect()
}

fn fields(declared: &DeclaredVariant, tag: &str) -> Vec<TtCompletion> {
    declared
        .constructors
        .iter()
        .find(|c| c.tag == tag)
        .and_then(|c| c.fields.as_deref())
        .unwrap_or_default()
        .iter()
        .map(|field| TtCompletion {
            label: field.name.clone(),
            kind: TtCompletionKind::Field,
            detail: super::names::field_signature(field),
            covered: false,
            range: None,
        })
        .collect()
}

fn text<'a>(source: &'a str, token: &Token) -> &'a str {
    &source[token.span.start..token.span.end]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(source: &str, needle: &str) -> Position {
        let offset = source.find(needle).expect("needle") + needle.len();
        let (line, character) = crate::lines::LineMap::lsp(source).utf16_position(offset);
        Position {
            line: line as u32,
            character: character as u32,
        }
    }

    fn labels(source: &str, needle: &str) -> Vec<String> {
        tt_completions_at(Path::new("/p/a.tt"), source, at(source, needle))
            .into_iter()
            .map(|item| item.label)
            .collect()
    }

    #[test]
    fn a_match_over_literals_offers_no_variant_tags() {
        let source = "variant Shape { Circle(r: number), Point }\n\
type Dir = \"north\" | \"south\";\n\
declare const d: Dir;\n\
const a = match (d) { \"north\" => 1, ‸ };\n\
const b = match (d) { 200 | 404 => 1, ‸ };\n\
const c = match (d) { is Error => 1, \"x\" => 2, ‸ };\n";
        for nth in 0..3 {
            let offset = source.match_indices('‸').nth(nth).unwrap().0;
            let text = source.replace('‸', "");
            let offset = offset - nth * '‸'.len_utf8();
            let (line, character) = crate::lines::LineMap::lsp(&text).utf16_position(offset);
            let position = Position {
                line: line as u32,
                character: character as u32,
            };
            let labels: Vec<String> = tt_completions_at(Path::new("/p/a.tt"), &text, position)
                .into_iter()
                .map(|item| item.label)
                .collect();
            assert_eq!(labels, vec!["_"], "arm {nth}");
        }
        assert_eq!(
            labels(
                "variant Shape { Circle(r: number), Point }\nconst e = match (s) { Circle(r) => r, ",
                "Circle(r) => r, "
            ),
            vec!["Circle", "Point", "_"]
        );
    }

    /// The member access at the end of `source`.
    fn member_at_end(path: &str, source: &str) -> Option<Option<String>> {
        member_access_at(Path::new(path), source, at(source, source)).map(|access| access.receiver)
    }

    #[test]
    fn a_tt_keyword_is_offered_where_its_construct_can_begin() {
        let keywords = |path: &str, source: &str| {
            let offset = source.find('‸').expect("cursor");
            let text = source.replace('‸', "");
            let (line, character) = crate::lines::LineMap::lsp(&text).utf16_position(offset);
            let position = Position {
                line: line as u32,
                character: character as u32,
            };
            tt_keywords_at(Path::new(path), &text, position)
                .into_iter()
                .map(TtKeyword::label)
                .collect::<Vec<_>>()
        };
        let statement = ["variant", "match", "try", "flow", "result", "let-else"];
        let expression = ["match", "flow", "result"];
        for (path, source, expected) in [
            ("/p/a.tt", "const x = 1;\n‸\n", &statement[..]),
            ("/p/a.tt", "const x = 1;\nma‸\n", &statement[..]),
            ("/p/a.tt", "function f() {\n  ‸\n}\n", &statement[..]),
            ("/p/a.tt", "const x = ‸;\n", &expression[..]),
            ("/p/a.tt", "const x = f(1, ma‸);\n", &expression[..]),
            ("/p/a.tt", "const o = { a: ‸ };\n", &expression[..]),
            ("/p/a.tt", "const f = (a: number) => ‸;\n", &expression[..]),
            ("/p/a.tt", "const s = `a ${‸}`;\n", &expression[..]),
            ("/p/a.ttx", "const e = <div>{‸}</div>;\n", &expression[..]),
            ("/p/a.tt", "export default ‸\n", &expression[..]),
            ("/p/a.tt", "export ‸\n", &["variant"][..]),
            ("/p/a.tt", "const cfg: Cfg = { ‸ };\n", &[][..]),
            ("/p/a.tt", "const cfg: Cfg = { a: 1, b‸ };\n", &[][..]),
            ("/p/a.tt", "import { ‸ } from \"./orders.tt\";\n", &[][..]),
            ("/p/a.ttx", "const e = <Row ‸ />;\n", &[][..]),
            ("/p/a.tt", "let y: ‸;\n", &[][..]),
            ("/p/a.tt", "class C {\n  ‸\n}\n", &[][..]),
            ("/p/a.tt", "const n = user.‸\n", &[][..]),
            ("/p/a.tt", "// ‸\n", &[][..]),
            ("/p/a.tt", "const s = \"‸\";\n", &[][..]),
            (
                "/p/a.tt",
                "variant V { A, B }\nconst v = match (x) { A => 1, ‸ };\n",
                &[][..],
            ),
        ] {
            assert_eq!(keywords(path, source), expected, "{source}");
        }
    }

    #[test]
    fn a_member_access_is_read_from_the_tokens_before_the_name() {
        for source in [
            "declare const nm: string;\nconst m = nm.trim().ma",
            "const t = foo().t",
            "const t = xs[0].t",
            "const t = \"abc\".len",
            "const t = k |> .t",
            "const t = k |> .",
            "const t = a?.b.",
            "const t = (a + b).",
        ] {
            assert_eq!(member_at_end("/p/a.tt", source), Some(None), "{source}");
        }
        for (source, receiver) in [
            ("const r = Result.", "Result"),
            ("const r = Result.O", "Result"),
            ("const r = Result?.O", "Result"),
            ("const r = ns.Shape.Ci", "ns.Shape"),
        ] {
            assert_eq!(
                member_at_end("/p/a.tt", source),
                Some(Some(receiver.to_string())),
                "{source}"
            );
        }
        // JSX text, attributes and generic arrows are not code; the
        // expression containers are.
        for (path, source, cursor, receiver) in [
            ("/p/a.tt", "const r = `${obj.na}`;", "obj.na", "obj"),
            ("/p/a.tt", "const s = `returned ${at.", "at.", "at"),
            ("/p/a.tt", "const s = `returned ${at.ge", "at.ge", "at"),
            ("/p/a.tt", "const s = `a ${x} b ${y.", "y.", "y"),
            (
                "/p/a.ttx",
                "const r = <div>{obj.na}</div>;",
                "obj.na",
                "obj",
            ),
            (
                "/p/a.ttx",
                "const el = <><p>{t.a}</p><p>{t.</p></>;",
                "{t.",
                "t",
            ),
            (
                "/p/a.ttx",
                "const el = <p>Don't {user.}</p>;",
                "user.",
                "user",
            ),
            (
                "/p/a.ttx",
                "const el = <a title=\"it's\" href='say \"hi\"' onClick={() => go(x.)}>{y.}</a>;",
                "x.",
                "x",
            ),
            (
                "/p/a.ttx",
                "const el = <a title=\"it's\" onClick={() => go(x.)}>{y.}</a>;",
                "y.",
                "y",
            ),
            (
                "/p/a.ttx",
                "const list = (\n  <ul>\n    {items.map(item => <li key={item.id}>{item.name} isn't {other.}</li>)}\n  </ul>\n);\n",
                "other.",
                "other",
            ),
            (
                "/p/a.ttx",
                "const id = <T,>(x: T) => x;\nconst pick = <T extends object>(x: T) => x;\nconst v = q.",
                "q.",
                "q",
            ),
            (
                "/p/a.ttx",
                "const el = <><img src='a.png' />{'literal'}{v.}</>;",
                "v.",
                "v",
            ),
        ] {
            let access = member_access_at(Path::new(path), source, at(source, cursor));
            assert_eq!(
                access.map(|access| access.receiver),
                Some(Some(receiver.to_string())),
                "{source}"
            );
        }
        let jsx_text = "const el = <p>see a.b</p>;";
        assert_eq!(
            member_access_at(Path::new("/p/a.ttx"), jsx_text, at(jsx_text, "a.b")),
            None
        );
        for source in [
            "const t = ma",
            "const t = [...xs",
            "const t = 1; // a.b",
            "const t = \"a.b",
            "const t = `a.b",
        ] {
            assert_eq!(member_at_end("/p/a.tt", source), None, "{source}");
        }
    }

    #[test]
    fn jsx_text_is_not_a_pattern_completion_context() {
        let source = format!("{DECL}const view = <div>if let </div>;");
        let items = tt_completions_at(Path::new("/p/a.ttx"), &source, at(&source, "if let "));
        assert!(items.is_empty(), "{items:?}");
    }

    const DECL: &str =
        "variant Shape { Circle(radius: number), Rect(w: number, h: number), Point }\n";

    #[test]
    fn an_arm_position_offers_the_subjects_cases() {
        // The arm being typed has no body yet — which is exactly when the
        // question is asked, and when the parser claims nothing.
        let src = format!("{DECL}const a = match (s) {{ Circle(r) => r, Po }};\n");
        assert_eq!(
            labels(&src, "Circle(r) => r, "),
            ["Circle", "Rect", "Point", "_"]
        );
        let items = tt_completions_at(Path::new("/p/a.tt"), &src, at(&src, "Circle(r) => r, "));
        assert_eq!(
            items
                .iter()
                .filter(|i| i.covered)
                .map(|i| i.label.as_str())
                .collect::<Vec<_>>(),
            ["Circle"],
            "an arm already written is marked, not hidden"
        );
    }

    #[test]
    fn a_guarded_arm_does_not_mark_its_case_covered() {
        let src = format!("{DECL}const a = match (s) {{ Circle(r) if r > 1 => 1, Po }};\n");
        let items = tt_completions_at(
            Path::new("/p/a.tt"),
            &src,
            at(&src, "Circle(r) if r > 1 => 1, "),
        );
        assert_eq!(
            items.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(),
            ["Circle", "Rect", "Point", "_"]
        );
        assert!(items.iter().all(|i| !i.covered), "{items:?}");

        let src = format!(
            "{DECL}const a = match (s) {{ Circle(r) if r > 1 => 1, Circle(r) => 0, Po }};\n"
        );
        let items = tt_completions_at(Path::new("/p/a.tt"), &src, at(&src, "Circle(r) => 0, "));
        assert_eq!(
            items
                .iter()
                .filter(|i| i.covered)
                .map(|i| i.label.as_str())
                .collect::<Vec<_>>(),
            ["Circle"]
        );
    }

    #[test]
    fn a_nested_pattern_does_not_mark_its_case_covered() {
        let decl = "variant O { Some(value: number), None }\nvariant W { Has(o: O), Nope }\ndeclare const w: W;\n";
        let src = format!("{decl}const a = match (w) {{ Has(o: Some(value)) => 1, No }};\n");
        let items = tt_completions_at(
            Path::new("/p/a.tt"),
            &src,
            at(&src, "Has(o: Some(value)) => 1, "),
        );
        assert!(items.iter().all(|i| !i.covered), "{items:?}");

        let src = format!("{decl}const a = match (w) {{ Has(o: renamed) => 1, No }};\n");
        let items = tt_completions_at(
            Path::new("/p/a.tt"),
            &src,
            at(&src, "Has(o: renamed) => 1, "),
        );
        assert_eq!(
            items
                .iter()
                .filter(|i| i.covered)
                .map(|i| i.label.as_str())
                .collect::<Vec<_>>(),
            ["Has"]
        );
    }

    #[test]
    fn a_payload_position_offers_the_cases_fields() {
        let src = format!("{DECL}const a = match (s) {{ Rect(w) => w, Point => 0 }};\n");
        assert_eq!(labels(&src, "{ Rect("), ["w", "h"]);
        // ...and after a comma inside the list, less the fields it binds.
        let src = format!("{DECL}const a = match (s) {{ Rect(w, ) => w }};\n");
        assert_eq!(labels(&src, "Rect(w, "), ["h"]);
    }

    #[test]
    fn a_payload_position_leaves_out_the_fields_already_bound() {
        for (source, needle, expected) in [
            ("Rect(w: width, ) => 0", "Rect(w: width, ", vec!["h"]),
            ("Rect(w: Some(v), ) => 0", "Rect(w: Some(v), ", vec!["h"]),
            // The name being typed is not yet bound.
            ("Rect(w, h) => 0", "Rect(w, h", vec!["h"]),
        ] {
            let source = format!("{DECL}const a = match (s) {{ {source} }};\n");
            assert_eq!(labels(&source, needle), expected, "{source}");
        }
    }

    #[test]
    fn an_if_let_offers_every_visible_case() {
        let src = format!("{DECL}if let  = s {{ }}\n");
        let found = labels(&src, "if let ");
        assert!(found.contains(&"Circle".to_string()), "{found:?}");
        // The built-ins are visible without a declaration.
        assert!(found.contains(&"Some".to_string()), "{found:?}");
    }

    #[test]
    fn a_nested_position_offers_the_fields_variant() {
        let src = "variant Inner { Yes(n: number), No }\n\
                   variant Outer { Wrap(inner: Inner), Bare }\n\
                   const a = match (o) { Wrap(inner: ) => 0 };\n";
        assert_eq!(labels(src, "match (o) { Wrap(inner: "), ["Yes", "No"]);
    }

    #[test]
    fn payload_positions_work_in_every_construct_that_has_a_pattern() {
        // The paren logic is construct-agnostic: what decides is that the
        // innermost open paren follows a case tag.
        let src = format!("{DECL}if let Rect() = s {{ }}\n");
        assert_eq!(labels(&src, "if let Rect("), ["w", "h"]);

        let src = format!("{DECL}const Rect() = s else {{ return; }};\n");
        assert_eq!(labels(&src, "const Rect("), ["w", "h"]);

        let src = "variant Inner { Yes(n: number), No }\n\
                   variant Outer { Wrap(inner: Inner), Bare }\n\
                   if let Wrap(inner: ) = o { }\n";
        assert_eq!(labels(src, "if let Wrap(inner: "), ["Yes", "No"]);
    }

    #[test]
    fn the_wildcard_is_an_arm_position_only() {
        let src = format!("{DECL}const a = match (s) {{  }};\n");
        assert!(labels(&src, "match (s) { ").contains(&"_".to_string()));
        let src = format!("{DECL}if let  = s {{ }}\n");
        assert!(!labels(&src, "if let ").contains(&"_".to_string()));
    }

    #[test]
    fn every_prefix_retains_cases_without_arbitrary_variant_selection() {
        let prefix =
            "variant Other { Wrong }\nvariant User { Admin(name: string, level: number), Guest }\n";
        for arm in ["", "A", "Ad", "Admin", "G", "Gu", "Guest"] {
            let source = format!("{prefix}const r = match (user) {{ {arm}");
            let found = labels(&source, &format!("match (user) {{ {arm}"));
            assert!(found.contains(&"Admin".into()), "{arm}: {found:?}");
            assert!(found.contains(&"Guest".into()), "{arm}: {found:?}");
        }
    }

    #[test]
    fn wildcard_and_unfinished_siblings_do_not_remove_case_candidates() {
        for sibling in ["_ => 0", "Unfin", "Guest => 0"] {
            let source = format!(
                "{DECL}variant User {{ Admin(name: string), Guest }}\nconst r = match (user) {{ Admin(name) => name, Gu, {sibling} }};"
            );
            let found = labels(&source, "name, Gu");
            assert_eq!(found, ["Admin", "Guest", "_"], "{sibling}");
        }
    }

    #[test]
    fn expression_pipes_do_not_become_pattern_evidence() {
        let source = format!("{DECL}const r = match (s) {{ Circle(r) => r | mask, Po }};");
        assert_eq!(
            labels(&source, "mask, Po"),
            ["Circle", "Rect", "Point", "_"]
        );
        let source = format!("{DECL}const r = match (s) {{ Circle(r) if r | mask => r, Po }};");
        assert_eq!(labels(&source, "r, Po"), ["Circle", "Rect", "Point", "_"]);
    }

    #[test]
    fn ambiguous_tags_preserve_fields_and_deduplicate_insertions() {
        let source = "variant Left { Shared(common: string, left: number) }\nvariant Right { Shared(common: number, right: boolean) }\nconst r = match (x) { Shared( }";
        assert_eq!(labels(source, "Shared( }"), Vec::<String>::new());
        assert_eq!(
            labels(source, "match (x) { Shared("),
            ["common", "left", "right"]
        );
    }

    #[test]
    fn tuple_slots_and_partial_payload_fields_are_completable() {
        for pattern in ["(", "(Ci", "(Circle(r), ", "(Circle(r), Po"] {
            let source = format!("{DECL}const r = match (a, b) {{ {pattern}");
            let found = labels(&source, &format!("match (a, b) {{ {pattern}"));
            assert!(found.contains(&"Circle".into()), "{pattern}: {found:?}");
            assert!(found.contains(&"Point".into()), "{pattern}: {found:?}");
        }
        for (pattern, expected) in [
            ("Rect(", vec!["w", "h"]),
            ("Rect(w", vec!["w", "h"]),
            ("Rect(w, ", vec!["h"]),
            ("Rect(w, h", vec!["h"]),
        ] {
            let source = format!("{DECL}const r = match (s) {{ {pattern}");
            assert_eq!(
                labels(&source, &format!("match (s) {{ {pattern}")),
                expected,
                "{pattern}"
            );
        }
    }

    #[test]
    fn nothing_is_offered_outside_a_pattern() {
        let src = format!("{DECL}const a = match (s) {{ Circle(r) => r }};\nconst b = ;\n");
        assert!(labels(&src, "const b = ").is_empty());
        // The scrutinee is an expression, not a pattern.
        assert!(labels(&src, "match (").is_empty());
        // An arm body is an expression too.
        assert!(labels(&src, "Circle(r) => ").is_empty());
    }

    #[test]
    fn call_arguments_are_not_payload_positions() {
        for (source, needle) in [
            (
                format!("{DECL}const c = Shape.Circle();\n"),
                "Shape.Circle(",
            ),
            (
                format!("{DECL}const c = Shape.Rect(w, );\n"),
                "Shape.Rect(w, ",
            ),
            (
                "import { Option } from \"tt/std/option\";\nconst o = Option.Some();\n".to_string(),
                "Option.Some(",
            ),
            (
                format!("{DECL}function Rect(a: number, b: number) {{}}\nRect(q, );\n"),
                "\nRect(q, ",
            ),
            (format!("{DECL}const c = Rect(q, "), "Rect(q, "),
            (format!("{DECL}Rect(q, "), "\nRect(q, "),
            (
                format!(
                    "{DECL}const a = match (s) {{ Circle(radius) => Rect(radius, ), _ => 0 }};\n"
                ),
                "=> Rect(radius, ",
            ),
            (
                format!("{DECL}const a = match (s) {{ Circle(radius) => Rect(radius, "),
                "=> Rect(radius, ",
            ),
            (
                format!("{DECL}const a = match (s) {{ Circle(radius) if Rect( => 1 }};\n"),
                "if Rect(",
            ),
            (
                format!("{DECL}if let Circle(radius) = Rect() {{ }}\n"),
                "= Rect(",
            ),
            (
                format!("{DECL}const Circle(radius) = Rect() else {{ return; }};\n"),
                "= Rect(",
            ),
        ] {
            assert!(
                labels(&source, needle).is_empty(),
                "{needle}: {:?}",
                labels(&source, needle)
            );
        }
    }

    #[test]
    fn a_variant_declaration_is_not_a_payload_position() {
        assert!(labels(DECL, "variant Shape { Circle(").is_empty());
        assert!(labels(DECL, "Rect(w: number, ").is_empty());
    }

    #[test]
    fn payload_positions_follow_the_pattern_that_introduces_them() {
        for (source, needle) in [
            (
                format!(
                    "{DECL}const a = match (s) {{ Circle(radius) => match (s) {{ Rect() => 1, _ => 2 }}, _ => 0 }};\n"
                ),
                "{ Rect(",
            ),
            (
                format!("{DECL}const a = match (s) {{ Circle(radius) => 1, Rect(w, "),
                "Rect(w, ",
            ),
            (
                format!("{DECL}const a = match (s) {{ Circle(radius) | Rect( => 1 }};\n"),
                "| Rect(",
            ),
            (format!("{DECL}if let Circle(radius) | Rect("), "| Rect("),
            (format!("{DECL}let Rect(w, "), "let Rect(w, "),
        ] {
            // A field the payload already binds is not offered again.
            let expected: &[&str] = if needle.ends_with("(w, ") {
                &["h"]
            } else {
                &["w", "h"]
            };
            assert_eq!(labels(&source, needle), expected, "{needle}");
        }
    }

    #[test]
    fn arm_bodies_and_guards_are_expressions() {
        let head = format!("{DECL}const a = match (s) {{ Circle(radius) ");
        for (arm, needle) in [
            ("=> [1, ], _ => 2 };\n", "=> [1, "),
            ("=> [1, ", "=> [1, "),
            ("=> radius | , _ => 2 };\n", "radius | "),
            ("=> radius | ", "radius | "),
            ("=> radius || ", "radius || "),
            ("=> (x: number, ) => 1, _ => 2 };\n", "(x: number, "),
            ("=> (x: number, ", "(x: number, "),
            ("=> ({ a: 1, }), _ => 2 };\n", "{ a: 1, "),
            ("=> { return [radius, ]; }, _ => 2 };\n", "[radius, "),
            ("=> { let x = 1, }, _ => 2 };\n", "let x = 1, "),
            ("=> { if (radius) { } }, _ => 2 };\n", "if (radius) { "),
            ("if radius | ", "radius | "),
            ("if radius > 0 => 1, Rect(w, h) if w > h ", "w > h "),
            ("=> , _ => 2 };\n", "radius) => "),
            ("=>  };\n", "radius) => "),
            ("=>\n  };\n", "radius) =>\n  "),
            ("=> ", "radius) => "),
            ("if radius > 0 =>  };\n", "radius > 0 => "),
        ] {
            let source = format!("{head}{arm}");
            assert!(
                labels(&source, needle).is_empty(),
                "{source}: {:?}",
                labels(&source, needle)
            );
        }
        let source = format!("{head}=> [1, 2], ");
        assert_eq!(
            labels(&source, "[1, 2], "),
            ["Circle", "Rect", "Point", "_"],
            "a finished arm's comma still opens the next arm"
        );
        let source = format!("{head}=> {{ return 1; }}, ");
        assert_eq!(labels(&source, "}, "), ["Circle", "Rect", "Point", "_"]);
    }

    #[test]
    fn an_identifier_scrutinee_match_has_arm_positions() {
        for source in [
            format!("{DECL}const a = match s {{  }};\n"),
            format!("{DECL}const a = match s {{ "),
        ] {
            let found = labels(&source, "match s { ");
            assert!(found.contains(&"Circle".to_string()), "{found:?}");
            assert!(found.contains(&"_".to_string()), "{found:?}");
        }
        for (source, needle) in [
            (
                format!("{DECL}const a = match s {{ Circle(r) => r, Po }};\n"),
                "=> r, ",
            ),
            (
                format!("{DECL}const a = match s {{ Circle(r) => r, Po"),
                "=> r, ",
            ),
        ] {
            assert_eq!(
                labels(&source, needle),
                ["Circle", "Rect", "Point", "_"],
                "{source}"
            );
        }
        let source = format!("{DECL}const a = match s {{ Rect(w, ");
        assert_eq!(labels(&source, "Rect(w, "), ["h"]);
        for (source, needle) in [
            (
                format!("{DECL}const a = match s {{ Circle(r) => Rect(r, "),
                "Rect(r, ",
            ),
            (
                format!("{DECL}const a = match s {{ Circle(r) if r | "),
                "r | ",
            ),
            (format!("{DECL}const a = match s"), "match s"),
        ] {
            assert!(labels(&source, needle).is_empty(), "{source}");
        }
    }

    #[test]
    fn an_unclosed_body_keeps_its_finished_arms_as_evidence() {
        let src = format!("{DECL}const a = match (s) {{ Circle(r) => r, ");
        let items = tt_completions_at(Path::new("/p/a.tt"), &src, at(&src, "=> r, "));
        assert_eq!(
            items
                .iter()
                .filter(|i| i.covered)
                .map(|i| i.label.as_str())
                .collect::<Vec<_>>(),
            ["Circle"]
        );
        let src = format!("{DECL}const a = match (s) {{ Circle(r) if r > 1 => 1, ");
        let items = tt_completions_at(Path::new("/p/a.tt"), &src, at(&src, "=> 1, "));
        assert!(items.iter().all(|i| !i.covered), "{items:?}");
        let src = format!("{DECL}const a = match (s) {{ Circle(r) => {{ return r; }}, Rect(w, ");
        assert_eq!(labels(&src, "Rect(w, "), ["h"]);
        let src = "variant Inner { Yes(n: number), No }\n\
                   variant Outer { Wrap(inner: Inner), Bare }\n\
                   const a = match (o) { Wrap(inner) => match (inner) { Yes(n) => n, ";
        assert_eq!(labels(src, "=> n, "), ["Yes", "No", "_"]);
        let src = format!("{DECL}const a = match (s) {{ Circle(r) => {{ if let ");
        let found = labels(&src, "if let ");
        assert!(found.contains(&"Circle".to_string()), "{found:?}");
        assert!(!found.contains(&"_".to_string()), "{found:?}");
    }

    #[test]
    fn only_tag_patterns_are_arm_evidence() {
        let src = format!("{DECL}const a = match (s) {{ is Circle => 1, Po");
        let found = labels(&src, "1, ");
        assert!(found.contains(&"Circle".to_string()), "{found:?}");
        assert!(found.contains(&"Point".to_string()), "{found:?}");
    }

    #[test]
    fn comments_and_literals_are_not_completion_sites() {
        let head = format!("{DECL}const a = match (s) {{ Circle(radius) => 1,");
        for (rest, needle) in [
            ("\n  // note: Rect(\n  _ => 2 };\n", "// note: Rect("),
            ("\n  // note: ", "// note: "),
            (" /* Rect( */ _ => 2 };\n", "/* Rect("),
            (" /* ", "/* "),
            (" _ => \"Rect(\" };\n", "\"Rect("),
            (" _ => `x ${1}, ` };\n", "${1}, "),
            (" _ => /Rect(/ };\n", "/Rect("),
        ] {
            let source = format!("{head}{rest}");
            assert!(
                labels(&source, needle).is_empty(),
                "{source}: {:?}",
                labels(&source, needle)
            );
        }
        let source = format!("{DECL}// if let \nconst t = \"if let \";\n");
        assert!(labels(&source, "// if let ").is_empty());
        assert!(labels(&source, "\"if let ").is_empty());
        let source = format!("{head} /* note */ ");
        assert_eq!(
            labels(&source, "/* note */ "),
            ["Circle", "Rect", "Point", "_"],
            "a closed comment ends before the arm slot"
        );
        let source = format!("{head} // note\n  ");
        assert_eq!(
            labels(&source, "// note\n  "),
            ["Circle", "Rect", "Point", "_"]
        );
    }
}
