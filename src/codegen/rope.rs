//! Mapping-aware output assembly for codegen.
//!
//! Emission builds a [`Rope`] — a sequence of glue, source, mark, and anchor
//! pieces. Finalization assigns every text piece an exact, construct, or
//! synthetic provenance in [`TargetFile`], validates the target, then prints
//! the code and language-tooling metadata. Byte-identical output remains the
//! invariant (TASK-050, TASK-158).
//!
//! Pieces *borrow*: a source piece is a `&str` into the original source and
//! a literal piece is usually a `&'static str`, so building a rope copies
//! no text at all — the single copy happens in [`Rope::flatten`], into an
//! output buffer pre-sized from the running byte length (TASK-056).

mod builder;

#[cfg(test)]
mod tests;

use std::borrow::Cow;

use crate::ice::{InternalCompilerError, Invariant, LoweringStage, LoweringSubject};
use crate::program_syntax::SourceSpan;
use crate::{
    AnchorKind, BindingOccurrence, DeclaredName, EmitAnchor, EmitMapping, PayloadTemp,
    ResultReturnTemp, ScrutineeTemp, SharedBinding, SourceKind,
};

pub(crate) use builder::{Flat, Rope};

/// What a [`Piece::Mark`] marks — the values codegen writes that a
/// type checker can be *asked about*, each paired with the source
/// construct it stands for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkKind {
    /// A `match`'s scrutinee temporary ([`crate::ScrutineeTemp`]).
    Scrutinee,
    /// End of an unannotated generated value declaration identifier.
    ContextualSlot,
    /// The same, for storage holding the index of the arm a dispatch
    /// selected rather than a value.
    SelectorSlot,
    OperandSlot,
    AssertedAnnotationEnd,
    /// The receiver a nested pattern tests ([`crate::PayloadTemp`]).
    Payload,
    /// Start of a value explicitly returned from a `result` block.
    ResultReturnStart,
    /// End of the same returned value.
    ResultReturnEnd,
    DeclaredNameStart,
    DeclaredNameEnd,
    DestructuredListStart,
    DestructuredListEnd,
    SharedBindingStart,
    SharedBindingOccurrence {
        end: usize,
        shorthand: bool,
    },
    SharedBindingEnd,
    /// The glue after it is written for the source at `src`, a construct
    /// part that starts there (a match arm), so the printer lays it out as
    /// that source's: on one line with it when a directive governs that line.
    SourcePoint,
    /// Start of glue written at source point `src` ([`crate::InsertedGlue`]).
    InsertedStart,
    /// End of the same glue.
    InsertedEnd,
}

enum Piece<'a> {
    /// Compiler-written glue (region scaffolding, destructurings, labels).
    Lit(Cow<'a, str>),
    /// Text copied from the source, starting at source byte offset `src`.
    Src { text: &'a str, src: usize },
    /// A zero-length note about the *next* byte the rope emits: the name
    /// codegen is about to write stands for the construct at source offset
    /// `src`. Carries no text, so it changes nothing about the output.
    Mark { src: usize, kind: MarkKind },
    /// A hard line break in generated glue: a newline, the enclosing layout
    /// scope's base indentation, and `depth` further indentation units.
    /// The text is resolved when the target is printed, because the base is
    /// the indentation of the output line the scope opened on.
    Break { depth: u16 },
    /// Opens a layout scope. The printer reads the base indentation off the
    /// line it opens on, so a lowering lays its glue out from the line the
    /// construct sits on. Nests.
    ScopeOpen,
    /// Closes the innermost open layout scope.
    ScopeClose,
    /// A zero-length note that everything up to the matching [`Piece::Close`]
    /// is glue one construct wrote ([`EmitAnchor`]). Nests.
    Open {
        src: usize,
        src_end: usize,
        owner_end: usize,
        context: Option<(usize, usize)>,
        kind: AnchorKind,
    },
    /// Closes the innermost open anchor.
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceOrigin {
    Construct {
        src: usize,
        src_end: usize,
        owner_end: usize,
        kind: AnchorKind,
    },
    Synthetic {
        parent: ExactOrigin,
        reason: SyntheticReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExactOrigin {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyntheticReason {
    UnanchoredGenerated,
    /// The `;` that keeps a statement the source ended by automatic
    /// semicolon insertion from running into generated text after it
    /// ([`TargetFile::separate_statements`]).
    StatementSeparator,
}

enum TargetPiece<'a> {
    Generated {
        text: Cow<'a, str>,
        origin: SourceOrigin,
    },
    Source {
        text: &'a str,
        origin: ExactOrigin,
    },
    Mark {
        src: usize,
        kind: MarkKind,
    },
    Break {
        depth: u16,
    },
    ScopeOpen,
    ScopeClose,
    Open {
        src: usize,
        src_end: usize,
        owner_end: usize,
        context: Option<(usize, usize)>,
        kind: AnchorKind,
    },
    Close,
}

impl TargetPiece<'_> {
    /// The piece's *fixed* text. A [`TargetPiece::Break`] has none — its
    /// text is resolved against the layout scope while printing.
    fn text(&self) -> &str {
        match self {
            TargetPiece::Generated { text, .. } => text,
            TargetPiece::Source { text, .. } => text,
            TargetPiece::Mark { .. }
            | TargetPiece::Break { .. }
            | TargetPiece::ScopeOpen
            | TargetPiece::ScopeClose
            | TargetPiece::Open { .. }
            | TargetPiece::Close => "",
        }
    }
}

struct TargetFile<'a> {
    pieces: Vec<TargetPiece<'a>>,
    len: usize,
    source_len: usize,
    /// The original source, when the caller can supply it — required for
    /// the preservation check's whitespace classification.
    source: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TargetError {
    LengthMismatch {
        expected: usize,
        actual: usize,
    },
    SourceOutOfBounds {
        start: usize,
        end: usize,
    },
    CloseWithoutOpen,
    UnclosedAnchors {
        count: usize,
    },
    /// A line break with no layout scope to indent from — the emitter that
    /// wrote it never said where its block structure starts, so the break
    /// would silently fall back to column 0.
    BreakOutsideScope,
    ScopeCloseWithoutOpen,
    UnclosedScopes {
        count: usize,
    },
}

/// Which source bytes the compiler does *not* own, and what the plan did
/// to them — the inputs of the target's preservation check.
///
/// Computed from the Core IR and the lowering plan — never from the output.
#[derive(Debug, Default)]
pub(crate) struct SourcePreservation {
    /// The pass-through ranges: source the compiler does not interpret
    /// (Core `Opaque` statements and expressions, template raw parts).
    /// Every non-whitespace byte in them must reach the target exactly
    /// once, in source order. Bytes outside them belong to tt constructs,
    /// whose lowerings rearrange, repeat, or drop their own text freely
    /// (a dropped `try` keyword, a binding name written per alternative).
    pub(crate) owned: Vec<SourceSpan>,
    /// Pass-through ranges the plan relocates (schedule captures): still
    /// printed exactly once, but at the capture site rather than in source
    /// order.
    pub(crate) relocated: Vec<SourceSpan>,
    /// Pass-through ranges a lowering claimed and rewrote — the frame of a
    /// block arm's `return` becoming a continuation assignment
    /// ([`crate::program_syntax::HostExit`]). Exempt from the exactly-once
    /// rule; the claim is recorded in the plan, never inferred here.
    pub(crate) rewritten: Vec<SourceSpan>,
}

impl SourcePreservation {
    fn index(spans: &[SourceSpan]) -> crate::span_index::SpanIndex {
        crate::span_index::SpanIndex::new(spans.iter().map(|span| (span.start, span.end)))
    }
}

impl TargetError {
    /// The broken contract, as a structured internal compiler error.
    fn into_ice(self) -> InternalCompilerError {
        let stage = LoweringStage::TargetOrigin;
        let subject = LoweringSubject::default();
        match self {
            TargetError::LengthMismatch { .. } => {
                InternalCompilerError::new(stage, Invariant::TargetLengthMismatch, subject)
            }
            TargetError::SourceOutOfBounds { start, end } => {
                InternalCompilerError::new(stage, Invariant::OriginOutOfBounds, subject)
                    .at(SourceSpan { start, end })
            }
            TargetError::CloseWithoutOpen | TargetError::UnclosedAnchors { .. } => {
                InternalCompilerError::new(stage, Invariant::OriginNesting, subject)
            }
            TargetError::BreakOutsideScope
            | TargetError::ScopeCloseWithoutOpen
            | TargetError::UnclosedScopes { .. } => {
                InternalCompilerError::new(stage, Invariant::LayoutScopeMissing, subject)
            }
        }
    }
}

impl<'a> TargetFile<'a> {
    fn from_rope(rope: Rope<'a>, source_len: usize) -> Self {
        let mut source_hints = vec![None; rope.pieces.len()];
        let mut nearest = None;
        for (index, piece) in rope.pieces.iter().enumerate() {
            if let Piece::Src { text, src } = piece {
                nearest = Some(ExactOrigin {
                    start: *src,
                    end: src.saturating_add(text.len()),
                });
            }
            source_hints[index] = nearest;
        }
        nearest = None;
        for (index, piece) in rope.pieces.iter().enumerate().rev() {
            if let Piece::Src { text, src } = piece {
                nearest = Some(ExactOrigin {
                    start: *src,
                    end: src.saturating_add(text.len()),
                });
            }
            if source_hints[index].is_none() {
                source_hints[index] = nearest;
            }
        }
        let mut pieces = Vec::with_capacity(rope.pieces.len());
        let mut origins = Vec::new();
        for (index, piece) in rope.pieces.into_iter().enumerate() {
            match piece {
                Piece::Lit(text) => pieces.push(TargetPiece::Generated {
                    text,
                    origin: origins.last().copied().unwrap_or(SourceOrigin::Synthetic {
                        parent: source_hints[index].unwrap_or(ExactOrigin {
                            start: 0,
                            end: source_len,
                        }),
                        reason: SyntheticReason::UnanchoredGenerated,
                    }),
                }),
                Piece::Src { text, src } => pieces.push(TargetPiece::Source {
                    text,
                    origin: ExactOrigin {
                        start: src,
                        end: src.saturating_add(text.len()),
                    },
                }),
                Piece::Mark { src, kind } => pieces.push(TargetPiece::Mark { src, kind }),
                Piece::Break { depth } => pieces.push(TargetPiece::Break { depth }),
                Piece::ScopeOpen => pieces.push(TargetPiece::ScopeOpen),
                Piece::ScopeClose => pieces.push(TargetPiece::ScopeClose),
                Piece::Open {
                    src,
                    src_end,
                    owner_end,
                    context,
                    kind,
                } => {
                    origins.push(SourceOrigin::Construct {
                        src,
                        src_end,
                        owner_end,
                        kind,
                    });
                    pieces.push(TargetPiece::Open {
                        src,
                        src_end,
                        owner_end,
                        context,
                        kind,
                    });
                }
                Piece::Close => {
                    origins.pop();
                    pieces.push(TargetPiece::Close);
                }
            }
        }
        Self {
            pieces,
            len: rope.len,
            source_len,
            source: None,
        }
    }

    /// Keeps every statement boundary the source makes by automatic
    /// semicolon insertion a boundary of the target.
    ///
    /// A source piece that carries the line break an automatic semicolon
    /// stands on and runs up to the next statement's start copies the
    /// source's separation of two statements. When the target continues
    /// with anything but that statement's own source text, what follows is
    /// generated or moved: a lowering's first token. When that token would continue the previous statement
    /// ([`crate::lexer::continues_statement`]), an explicit `;` is written
    /// between them. One rule covers every lowering, because it reads the
    /// target's pieces rather than the emitter path that produced them.
    fn separate_statements(&mut self, boundaries: &[usize], kind: SourceKind) {
        if boundaries.is_empty() {
            return;
        }
        let mut separators = Vec::new();
        for (index, piece) in self.pieces.iter().enumerate() {
            let TargetPiece::Source { origin, text } = piece else {
                continue;
            };
            if boundaries.binary_search(&origin.end).is_err()
                || !crate::scanner::contains_line_terminator(text.as_bytes(), 0, text.len())
            {
                continue;
            }
            let following = self.pieces[index + 1..].iter().find(|piece| {
                !piece.text().is_empty() || matches!(piece, TargetPiece::Break { .. })
            });
            match following {
                None => continue,
                Some(TargetPiece::Source { origin: next, .. }) if next.start == origin.end => {
                    continue;
                }
                Some(_) => {}
            }
            if crate::lexer::continues_statement(&self.leading_text(index + 1), kind) {
                separators.push((index + 1, origin.end));
            }
        }
        for (index, boundary) in separators.into_iter().rev() {
            self.len += 1;
            self.pieces.insert(
                index,
                TargetPiece::Generated {
                    text: Cow::Borrowed(";"),
                    origin: SourceOrigin::Synthetic {
                        parent: ExactOrigin {
                            start: boundary,
                            end: boundary,
                        },
                        reason: SyntheticReason::StatementSeparator,
                    },
                },
            );
        }
    }

    /// The target's text from piece `from` through the end of the line its
    /// first significant byte is on, with each layout break read as the
    /// line break it prints.
    fn leading_text(&self, from: usize) -> String {
        let mut text = String::new();
        for piece in &self.pieces[from..] {
            let chunk = match piece {
                TargetPiece::Break { .. } => "\n",
                piece => piece.text(),
            };
            let bytes = text.as_bytes();
            let (start, _) = crate::scanner::skip_trivia(bytes, 0, bytes.len());
            let from = if start < bytes.len() {
                0
            } else {
                let chunk_bytes = chunk.as_bytes();
                crate::scanner::skip_trivia(chunk_bytes, 0, chunk_bytes.len()).0
            };
            let chunk_bytes = chunk.as_bytes();
            let stop = crate::scanner::line_end(chunk_bytes, from, chunk_bytes.len());
            text.push_str(&chunk[..stop]);
            if stop < chunk_bytes.len() {
                break;
            }
        }
        text
    }

    fn validate(&self) -> Result<(), TargetError> {
        let actual = self.pieces.iter().map(|piece| piece.text().len()).sum();
        if actual != self.len {
            return Err(TargetError::LengthMismatch {
                expected: self.len,
                actual,
            });
        }
        let mut open = 0usize;
        let mut scopes = 0usize;
        for piece in &self.pieces {
            match piece {
                TargetPiece::Source {
                    origin: ExactOrigin { start, end },
                    ..
                } if start > end || *end > self.source_len => {
                    return Err(TargetError::SourceOutOfBounds {
                        start: *start,
                        end: *end,
                    });
                }
                TargetPiece::Open { .. } => open += 1,
                TargetPiece::Close if open == 0 => return Err(TargetError::CloseWithoutOpen),
                TargetPiece::Close => open -= 1,
                TargetPiece::ScopeOpen => scopes += 1,
                TargetPiece::ScopeClose if scopes == 0 => {
                    return Err(TargetError::ScopeCloseWithoutOpen);
                }
                TargetPiece::ScopeClose => scopes -= 1,
                TargetPiece::Break { .. } if scopes == 0 => {
                    return Err(TargetError::BreakOutsideScope);
                }
                TargetPiece::Generated { origin, .. } => match origin {
                    SourceOrigin::Construct {
                        src,
                        src_end,
                        owner_end,
                        ..
                    } if src > src_end || src_end > owner_end || *owner_end > self.source_len => {
                        return Err(TargetError::SourceOutOfBounds {
                            start: *src,
                            end: *owner_end,
                        });
                    }
                    SourceOrigin::Synthetic { parent, .. }
                        if parent.start > parent.end || parent.end > self.source_len =>
                    {
                        return Err(TargetError::SourceOutOfBounds {
                            start: parent.start,
                            end: parent.end,
                        });
                    }
                    SourceOrigin::Construct { .. } | SourceOrigin::Synthetic { .. } => {}
                },
                TargetPiece::Source { .. }
                | TargetPiece::Mark { .. }
                | TargetPiece::Break { .. } => {}
            }
        }
        if open != 0 {
            return Err(TargetError::UnclosedAnchors { count: open });
        }
        if scopes != 0 {
            return Err(TargetError::UnclosedScopes { count: scopes });
        }
        Ok(())
    }

    /// Checks that the target treats the pass-through bytes the way the
    /// contract says (`docs/design/program-lowering.md` §2, §11,
    /// `validate_source_preservation`): every non-whitespace byte the
    /// compiler does not own reaches the target exactly once — never
    /// twice, never silently dropped — and in source order, except inside
    /// a range the plan explicitly relocated.
    ///
    /// The check reads target pieces, source intervals, and the plan's
    /// facts — never the printed string.
    fn validate_source_preservation(
        &self,
        preservation: &SourcePreservation,
    ) -> Result<(), InternalCompilerError> {
        let stage = LoweringStage::TargetSourcePreservation;
        let subject = LoweringSubject::default();
        let owned = SourcePreservation::index(&preservation.owned);
        let relocated = SourcePreservation::index(&preservation.relocated);
        let rewritten = SourcePreservation::index(&preservation.rewritten);
        let mut printed = vec![0u16; self.source_len];
        let mut last_ordered: Option<(usize, usize)> = None;
        for piece in &self.pieces {
            let TargetPiece::Source {
                origin: ExactOrigin { start, end },
                ..
            } = piece
            else {
                continue;
            };
            for count in &mut printed[*start..(*end).min(self.source_len)] {
                *count = count.saturating_add(1);
            }
            // Order applies to the pass-through stream only: pieces inside
            // a construct's own text are its lowering's to arrange, and
            // pieces inside a relocated range were moved on purpose.
            if !owned.any_containing(*start) || relocated.any_containing(*start) {
                continue;
            }
            if let Some((previous_start, previous_end)) = last_ordered
                && *start < previous_end
            {
                return Err(
                    InternalCompilerError::new(stage, Invariant::SourceReordered, subject)
                        .at(SourceSpan {
                            start: *start,
                            end: *end,
                        })
                        .with_origin(vec![SourceSpan {
                            start: previous_start,
                            end: previous_end,
                        }]),
                );
            }
            last_ordered = Some((*start, *end));
        }
        // `flatten` sets the source before it validates, and this
        // validator only runs from there.
        let source = self
            .source
            .expect("flatten installs the source before validating against it");
        // Rope trimming follows `str::trim`, which recognizes Unicode
        // whitespace. A byte counts as whitespace when the scalar value it
        // belongs to does, so this validator uses the same classification,
        // including ASCII vertical tab and multibyte spaces. Classifying one
        // byte at a time would reject continuation bytes after trimming had
        // legitimately removed the complete character.
        let whitespace = |at: usize| {
            let start = (0..=at)
                .rev()
                .find(|&start| source.is_char_boundary(start))
                .unwrap_or(0);
            source[start..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
        };
        for span in &preservation.owned {
            let clipped = span.start..span.end.min(self.source_len);
            for (at, &count) in clipped.clone().zip(&printed[clipped]) {
                if count == 1 || (count == 0 && whitespace(at)) || rewritten.any_containing(at) {
                    continue;
                }
                let byte = SourceSpan {
                    start: at,
                    end: at + 1,
                };
                if count > 1 {
                    return Err(InternalCompilerError::new(
                        stage,
                        Invariant::SourceEmittedTwice,
                        subject,
                    )
                    .at(byte)
                    .with_origin(vec![*span]));
                }
                if count == 0 {
                    return Err(InternalCompilerError::new(
                        stage,
                        Invariant::SourceOmitted,
                        subject,
                    )
                    .at(byte)
                    .with_origin(vec![*span]));
                }
            }
        }
        Ok(())
    }

    fn print(self, newline: &str, governed: &[GovernedStatement]) -> Flat {
        let governed_line = |at: usize| {
            governed
                .iter()
                .any(|statement| statement.start <= at && at <= statement.line_end)
        };
        let mut next_source = vec![None; self.pieces.len()];
        let mut next_governed = vec![None; self.pieces.len()];
        let mut following = None;
        let mut following_governed = None;
        for (index, piece) in self.pieces.iter().enumerate().rev() {
            next_source[index] = following;
            next_governed[index] = following_governed;
            if let TargetPiece::Source {
                origin: ExactOrigin { start, .. },
                ..
            } = piece
            {
                following = Some(*start);
                if governed_line(*start) {
                    following_governed = Some(*start);
                }
            }
        }
        let mut previous_source_end: Option<usize> = None;
        let mut single_line_breaks = Vec::new();
        let mut out = String::with_capacity(self.len + self.len / 8);
        let mut scopes: Vec<String> = Vec::new();
        let mut mappings: Vec<EmitMapping> = Vec::new();
        let mut marks: Vec<ScrutineeTemp> = Vec::new();
        let mut payloads: Vec<PayloadTemp> = Vec::new();
        let mut result_returns: Vec<ResultReturnTemp> = Vec::new();
        let mut contextual_slots = Vec::new();
        let mut selector_slots = Vec::new();
        let mut operand_slots = Vec::new();
        let mut asserted_slots = Vec::new();
        let mut declared_names: Vec<DeclaredName> = Vec::new();
        let mut shared_bindings: Vec<SharedBinding> = Vec::new();
        let mut destructured_lists: Vec<crate::DestructuredList> = Vec::new();
        let mut anchors: Vec<EmitAnchor> = Vec::new();
        let mut inserted: Vec<crate::InsertedGlue> = Vec::new();
        let mut open: Vec<OpenAnchor> = Vec::new();
        for (index, piece) in self.pieces.iter().enumerate() {
            let single_line = match (
                previous_source_end,
                next_source[index],
                next_governed[index],
            ) {
                (Some(previous), Some(next), Some(line)) => governed.iter().any(|statement| {
                    statement.start <= previous
                        && previous <= statement.end
                        && statement.start <= next
                        && next <= statement.end
                        && statement.start <= line
                        && line <= statement.line_end
                }),
                _ => false,
            };
            match piece {
                TargetPiece::Mark {
                    src,
                    kind: MarkKind::InsertedStart,
                } => inserted.push(crate::InsertedGlue {
                    src: *src,
                    out: out.len(),
                    out_end: out.len(),
                }),
                TargetPiece::Mark {
                    src,
                    kind: MarkKind::InsertedEnd,
                } => {
                    inserted
                        .last_mut()
                        .filter(|glue| glue.src == *src && glue.out_end == glue.out)
                        .unwrap_or_else(|| crate::ice::bug!("inserted glue end has no start"))
                        .out_end = out.len();
                }
                TargetPiece::Open {
                    src,
                    src_end,
                    owner_end,
                    context,
                    kind,
                } => open.push(OpenAnchor {
                    out: out.len(),
                    src: *src,
                    src_end: *src_end,
                    owner_end: *owner_end,
                    context: *context,
                    kind: *kind,
                }),
                TargetPiece::Close => {
                    if let Some(OpenAnchor {
                        out: start,
                        src,
                        src_end,
                        owner_end,
                        context,
                        kind,
                    }) = open.pop()
                    {
                        anchors.push(EmitAnchor {
                            out: start,
                            end: out.len(),
                            src,
                            src_end,
                            owner_end,
                            context,
                            kind,
                        });
                    }
                }
                TargetPiece::Mark {
                    src,
                    kind: MarkKind::SourcePoint,
                } => previous_source_end = Some(*src),
                TargetPiece::Mark {
                    kind: MarkKind::ContextualSlot,
                    ..
                } => contextual_slots.push(out.len()),
                TargetPiece::Mark {
                    kind: MarkKind::SelectorSlot,
                    ..
                } => {
                    contextual_slots.push(out.len());
                    selector_slots.push(out.len());
                }
                TargetPiece::Mark {
                    kind: MarkKind::OperandSlot,
                    ..
                } => {
                    contextual_slots.push(out.len());
                    operand_slots.push(out.len());
                }
                TargetPiece::Mark {
                    kind: MarkKind::AssertedAnnotationEnd,
                    ..
                } => {
                    let slot = *contextual_slots.last().unwrap_or_else(|| {
                        crate::ice::bug!("an asserted annotation follows no value slot")
                    });
                    asserted_slots.push((slot, out.len()));
                }
                TargetPiece::Mark {
                    src,
                    kind: MarkKind::Scrutinee,
                } => marks.push(ScrutineeTemp {
                    src: *src,
                    out: out.len(),
                }),
                TargetPiece::Mark {
                    src,
                    kind: MarkKind::Payload,
                } => payloads.push(PayloadTemp {
                    src: *src,
                    out: out.len(),
                }),
                TargetPiece::Mark {
                    src,
                    kind: MarkKind::ResultReturnStart,
                } => result_returns.push(ResultReturnTemp {
                    src: *src,
                    src_end: *src,
                    out: out.len(),
                    out_end: out.len(),
                }),
                TargetPiece::Mark {
                    src,
                    kind: MarkKind::ResultReturnEnd,
                } => {
                    let mark = result_returns
                        .iter_mut()
                        .rev()
                        .find(|mark| mark.src == *src && mark.out_end == mark.out)
                        .unwrap_or_else(|| {
                            crate::ice::bug!("Result return end has no matching start")
                        });
                    mark.out_end = out.len();
                }
                TargetPiece::Mark {
                    src,
                    kind: MarkKind::DeclaredNameStart,
                } => declared_names.push(DeclaredName {
                    src: *src,
                    src_end: *src,
                    out: out.len(),
                    out_end: out.len(),
                }),
                TargetPiece::Mark {
                    src,
                    kind: MarkKind::DeclaredNameEnd,
                } => {
                    let name = declared_names
                        .last_mut()
                        .filter(|name| name.out_end == name.out && name.src <= *src)
                        .unwrap_or_else(|| {
                            crate::ice::bug!("declared name end has no matching start")
                        });
                    name.src_end = *src;
                    name.out_end = out.len();
                }
                TargetPiece::Mark {
                    src,
                    kind: MarkKind::DestructuredListStart,
                } => destructured_lists.push(crate::DestructuredList {
                    src: *src,
                    src_end: *src,
                    out: out.len(),
                    out_end: out.len(),
                }),
                TargetPiece::Mark {
                    src,
                    kind: MarkKind::DestructuredListEnd,
                } => {
                    let list = destructured_lists
                        .last_mut()
                        .filter(|list| list.out_end == list.out && list.src <= *src)
                        .unwrap_or_else(|| {
                            crate::ice::bug!("destructured list end has no matching start")
                        });
                    list.src_end = *src;
                    list.out_end = out.len();
                }
                TargetPiece::Mark {
                    kind: MarkKind::SharedBindingStart,
                    ..
                } => shared_bindings.push(SharedBinding {
                    out: out.len(),
                    out_end: out.len(),
                    occurrences: Vec::new(),
                }),
                TargetPiece::Mark {
                    src,
                    kind: MarkKind::SharedBindingOccurrence { end, shorthand },
                } => shared_bindings
                    .last_mut()
                    .filter(|binding| binding.out_end == binding.out)
                    .unwrap_or_else(|| {
                        crate::ice::bug!("shared binding occurrence has no open binding")
                    })
                    .occurrences
                    .push(BindingOccurrence {
                        src: *src,
                        src_end: *end,
                        shorthand: *shorthand,
                    }),
                TargetPiece::Mark {
                    kind: MarkKind::SharedBindingEnd,
                    ..
                } => {
                    shared_bindings
                        .last_mut()
                        .filter(|binding| binding.out_end == binding.out)
                        .unwrap_or_else(|| {
                            crate::ice::bug!("shared binding end has no matching start")
                        })
                        .out_end = out.len();
                }
                TargetPiece::ScopeOpen => scopes.push(line_indent(&out).to_owned()),
                TargetPiece::ScopeClose => {
                    scopes.pop();
                }
                TargetPiece::Break { .. } if single_line => {
                    single_line_breaks.push(out.len());
                    if !out.ends_with(' ') {
                        out.push(' ');
                    }
                }
                TargetPiece::Break { depth } => {
                    out.push_str(newline);
                    if let Some(base) = scopes.last() {
                        out.push_str(base);
                    }
                    for _ in 0..*depth {
                        out.push_str(INDENT);
                    }
                }
                TargetPiece::Generated { text, .. } if single_line && text.contains('\n') => {
                    single_line_breaks.push(out.len());
                    out.push_str(&single_line_text(text));
                }
                TargetPiece::Generated { text, .. } => push_generated(&mut out, text, newline),
                TargetPiece::Source {
                    text,
                    origin: ExactOrigin { start, .. },
                } => {
                    previous_source_end = Some(start + text.len());
                    let at = out.len();
                    if let Some(last) = mappings.last_mut()
                        && last.src + last.len == *start
                        && last.out + last.len == at
                    {
                        last.len += text.len();
                    } else {
                        mappings.push(EmitMapping {
                            src: *start,
                            out: at,
                            len: text.len(),
                        });
                    }
                    out.push_str(text);
                }
            }
        }
        if result_returns.iter().any(|mark| mark.out == mark.out_end) {
            crate::ice::bug!("Result return start has no matching end")
        }
        marks.sort_by_key(|mark| mark.out);
        payloads.sort_by_key(|mark| mark.out);
        result_returns.sort_by_key(|mark| mark.out);
        Flat {
            code: out,
            mappings,
            scrutinee_temps: marks,
            payload_temps: payloads,
            anchors,
            result_return_temps: result_returns,
            contextual_slots,
            selector_slots,
            operand_slots,
            asserted_slots,
            generated_names: std::collections::HashSet::new(),
            declared_names,
            shared_bindings,
            destructured_lists,
            inserted,
            support_imports: Vec::new(),
            commonjs: false,
            single_line_breaks,
        }
    }
}

/// One anchor mid-print: opened, not yet closed ([`EmitAnchor`] minus its
/// end).
struct OpenAnchor {
    out: usize,
    src: usize,
    src_end: usize,
    owner_end: usize,
    context: Option<(usize, usize)>,
    kind: AnchorKind,
}

/// One level of generated indentation.
const INDENT: &str = "  ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GovernedStatement {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) line_end: usize,
}

pub(crate) fn single_line_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut lines = text.split('\n');
    if let Some(first) = lines.next() {
        out.push_str(
            first
                .strip_suffix('\r')
                .unwrap_or(first)
                .trim_end_matches([' ', '\t']),
        );
    }
    for line in lines {
        let line = line
            .strip_suffix('\r')
            .unwrap_or(line)
            .trim_matches([' ', '\t']);
        if !out.ends_with(' ') {
            out.push(' ');
        }
        out.push_str(line);
    }
    out
}

fn push_generated(out: &mut String, text: &str, newline: &str) {
    if newline == "\n" {
        out.push_str(text);
        return;
    }
    let mut rest = text;
    while let Some(at) = rest.find('\n') {
        let line = &rest[..at];
        out.push_str(line.strip_suffix('\r').unwrap_or(line));
        out.push_str(newline);
        rest = &rest[at + 1..];
    }
    out.push_str(rest);
}

/// The whitespace a line starts with — the base a lowering's generated
/// block structure is laid out from.
fn line_indent(out: &str) -> &str {
    let line = &out[crate::lines::line_start_before(out, out.len())..];
    let end = line
        .find(|byte: char| byte != ' ' && byte != '\t')
        .unwrap_or(line.len());
    &line[..end]
}

impl<'a> Piece<'a> {
    fn text(&self) -> &str {
        match self {
            Piece::Lit(t) => t,
            Piece::Src { text, .. } => text,
            Piece::Mark { .. }
            | Piece::Break { .. }
            | Piece::ScopeOpen
            | Piece::ScopeClose
            | Piece::Open { .. }
            | Piece::Close => "",
        }
    }

    /// True for a piece that ends the output line it sits on. A break's
    /// text is resolved at print time, so it never reaches [`Piece::text`].
    fn ends_line(&self) -> bool {
        match self {
            Piece::Break { .. } => true,
            piece => crate::lines::ends_with_line_break(piece.text()),
        }
    }

    fn is_break(&self) -> bool {
        matches!(self, Piece::Break { .. })
    }

    /// True for a piece that carries text a caller can trim.
    fn is_text(&self) -> bool {
        matches!(self, Piece::Lit(_) | Piece::Src { .. })
    }

    /// Drops the first `cut` bytes (a char boundary) from the piece.
    fn cut_front(&mut self, cut: usize) {
        match self {
            Piece::Lit(Cow::Borrowed(t)) => *t = &t[cut..],
            Piece::Lit(Cow::Owned(t)) => drop(t.drain(..cut)),
            Piece::Src { text, src } => {
                *text = &text[cut..];
                *src += cut;
            }
            Piece::Mark { .. }
            | Piece::Break { .. }
            | Piece::ScopeOpen
            | Piece::ScopeClose
            | Piece::Open { .. }
            | Piece::Close => {}
        }
    }

    /// Keeps only the first `keep` bytes (a char boundary) of the piece.
    fn truncate(&mut self, keep: usize) {
        match self {
            Piece::Lit(Cow::Borrowed(t)) => *t = &t[..keep],
            Piece::Lit(Cow::Owned(t)) => t.truncate(keep),
            Piece::Src { text, .. } => *text = &text[..keep],
            Piece::Mark { .. }
            | Piece::Break { .. }
            | Piece::ScopeOpen
            | Piece::ScopeClose
            | Piece::Open { .. }
            | Piece::Close => {}
        }
    }
}
