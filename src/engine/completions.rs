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

use super::language::Position;

/// What a completion item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtCompletionKind {
    /// A case tag.
    Case,
    /// A payload field name.
    Field,
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
}

/// What can be written at `position`, or an empty list when the position is
/// not a pattern position tt owns.
///
/// The answer never includes ordinary TypeScript completions: those are the
/// service's, and a consumer merges the two lists.
pub fn tt_completions_at(path: &Path, source: &str, position: Position) -> Vec<TtCompletion> {
    let offset = super::language::source_byte(source, position);
    let (program, tokens) = crate::parser::lex_and_parse_with_kind(
        source,
        crate::SourceKind::from_path(path).unwrap_or_default(),
    );
    let declarations = super::language::analyses_for(path, source).declarations;
    let items = match context(source, &program, &tokens, offset) {
        Some(Context::Case { of: Some(arms) }) => {
            let mut items = resolve_all(&declarations, &arms.tags)
                .flat_map(|declared| cases(declared, &arms.covered))
                .collect::<Vec<_>>();
            // An arm position always admits the wildcard, whether or not
            // the subject resolved.
            items.push(TtCompletion {
                label: "_".to_string(),
                kind: TtCompletionKind::Wildcard,
                detail: "wildcard arm — every remaining case (must be last)".to_string(),
                covered: false,
            });
            items
        }
        // A pattern position with nothing to say which variant it is over —
        // an `if let` names its variant only by the tag being typed. Every
        // visible case is a candidate, and the editor filters by prefix.
        Some(Context::Case { of: None }) => declarations
            .iter()
            .flat_map(|declared| cases(declared, &[]))
            .collect(),
        Some(Context::Field { tag, written }) => {
            resolve_all(&declarations, std::slice::from_ref(&tag))
                .flat_map(|declared| fields(declared, &tag))
                .filter(|field| !written.contains(&field.label))
                .collect()
        }
        Some(Context::Nested { tag, field }) => {
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
                .collect()
        }
        None => Vec::new(),
    };
    merge_candidates(items)
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
#[derive(Debug, PartialEq, Eq)]
enum Context {
    /// A tag is expected. `of` carries the tags already written in the same
    /// match, which is what says *which* variant — `None` when the position
    /// has no such evidence (an `if let`).
    Case { of: Option<ArmTags> },
    /// A payload field name of `tag` is expected; `written` are the fields
    /// the same payload already binds, which a pattern may not repeat.
    Field { tag: String, written: Vec<String> },
    /// A nested pattern's tag is expected, in `tag`'s field `field`.
    Nested { tag: String, field: String },
}

fn context(source: &str, program: &Program, tokens: &[Token], offset: usize) -> Option<Context> {
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
    if before == 0 {
        return None;
    }
    let site = match parsed_at(program, offset) {
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
    site_context(source, tokens, site, before, prefix)
}

/// What the pattern grammar expects at `before`, inside `site`'s pattern.
fn site_context(
    source: &str,
    tokens: &[Token],
    site: PatternSite,
    before: usize,
    prefix: Option<usize>,
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
            });
        }
        return None;
    }
    // A tuple pattern's parens open the arm; each slot is a case position.
    if matches!(site, PatternSite::Arm { .. })
        && open == start
        && matches!(
            tokens[before - 1].kind,
            TokenKind::Punct(b'(' | b',' | b'|')
        )
    {
        return Some(Context::Case {
            of: Some(ArmTags {
                tags: Vec::new(),
                covered: Vec::new(),
            }),
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
    program
        .segments
        .iter()
        .find_map(|segment| segment_at(segment, offset))
}

fn segment_at(segment: &Segment, offset: usize) -> Option<Parsed> {
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
#[derive(Debug, PartialEq, Eq)]
struct ArmTags {
    tags: Vec<String>,
    covered: Vec<String>,
}

/// The tags the finished arms of the match body at `open` name, as the
/// parser reads their patterns. The tag being typed at `prefix` is not
/// evidence yet. A guarded or nested alternative names a tag without
/// covering it.
fn arm_tags(source: &str, tokens: &[Token], open: usize, prefix: Option<usize>) -> ArmTags {
    let prefix = prefix.map(|index| tokens[index].span.start);
    let mut tags = Vec::new();
    let mut covered: Vec<String> = Vec::new();
    for header in crate::parser::arm_headers(source, tokens, open) {
        let Some(Pattern::Tags(alternatives)) = header.pattern else {
            continue;
        };
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
    ArmTags { tags, covered }
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
    declarations.iter().find(|d| d.name == base)
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
        let src = format!("{DECL}const a = match (s) {{ is Circle => 1, 1 | 2 => 2, Po");
        let found = labels(&src, "2 => 2, ");
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
