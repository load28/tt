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

use std::borrow::Cow;

use crate::ice::{InternalCompilerError, Invariant, LoweringStage, LoweringSubject};
use crate::program_syntax::SourceSpan;
use crate::{AnchorKind, EmitAnchor, EmitMapping, PayloadTemp, ResultReturnTemp, ScrutineeTemp};

/// What a [`Piece::Mark`] marks — the values codegen writes that a
/// type checker can be *asked about*, each paired with the source
/// construct it stands for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkKind {
    /// A `match`'s scrutinee temporary ([`crate::ScrutineeTemp`]).
    Scrutinee,
    /// The receiver a nested pattern tests ([`crate::PayloadTemp`]).
    Payload,
    /// Start of a value explicitly returned from a `result` block.
    ResultReturnStart,
    /// End of the same returned value.
    ResultReturnEnd,
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
    fn owns(&self, at: usize) -> bool {
        self.owned
            .iter()
            .any(|span| span.start <= at && at < span.end)
    }

    fn relocates(&self, at: usize) -> bool {
        self.relocated
            .iter()
            .any(|span| span.start <= at && at < span.end)
    }

    fn rewrites(&self, at: usize) -> bool {
        self.rewritten
            .iter()
            .any(|span| span.start <= at && at < span.end)
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
            if !preservation.owns(*start) || preservation.relocates(*start) {
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
        for span in &preservation.owned {
            let clipped = span.start..span.end.min(self.source_len);
            for (at, &count) in clipped.clone().zip(&printed[clipped]) {
                if preservation.rewrites(at) {
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
                if count == 0 && !source.as_bytes()[at].is_ascii_whitespace() {
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

    fn print(self) -> Flat {
        let mut out = String::with_capacity(self.len + self.len / 8);
        let mut scopes: Vec<String> = Vec::new();
        let mut mappings: Vec<EmitMapping> = Vec::new();
        let mut marks: Vec<ScrutineeTemp> = Vec::new();
        let mut payloads: Vec<PayloadTemp> = Vec::new();
        let mut result_returns: Vec<ResultReturnTemp> = Vec::new();
        let mut anchors: Vec<EmitAnchor> = Vec::new();
        let mut open: Vec<OpenAnchor> = Vec::new();
        for piece in &self.pieces {
            match piece {
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
                TargetPiece::ScopeOpen => scopes.push(line_indent(&out).to_owned()),
                TargetPiece::ScopeClose => {
                    scopes.pop();
                }
                TargetPiece::Break { depth } => {
                    out.push('\n');
                    if let Some(base) = scopes.last() {
                        out.push_str(base);
                    }
                    for _ in 0..*depth {
                        out.push_str(INDENT);
                    }
                }
                TargetPiece::Generated { text, .. } => out.push_str(text),
                TargetPiece::Source {
                    text,
                    origin: ExactOrigin { start, .. },
                } => {
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

/// The whitespace a line starts with — the base a lowering's generated
/// block structure is laid out from.
fn line_indent(out: &str) -> &str {
    let line = match out.rfind('\n') {
        Some(newline) => &out[newline + 1..],
        None => out,
    };
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
            piece => piece.text().ends_with('\n'),
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

#[derive(Default)]
pub(crate) struct Rope<'a> {
    pieces: Vec<Piece<'a>>,
    /// Total byte length of the pieces — [`Rope::flatten`]'s exact capacity.
    len: usize,
}

impl<'a> Rope<'a> {
    pub(crate) fn new() -> Rope<'a> {
        Rope::default()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    pub(crate) fn push_lit(&mut self, text: impl Into<Cow<'a, str>>) {
        let text = text.into();
        if !text.is_empty() {
            self.len += text.len();
            self.pieces.push(Piece::Lit(text));
        }
    }

    /// Ends the current line of generated glue and opens the next one
    /// `depth` indentation units inside the enclosing layout scope. The
    /// whitespace itself is resolved when the target is printed
    /// ([`Rope::scoped`]), because it depends on where the scope opened.
    pub(crate) fn push_break(&mut self, depth: u16) {
        self.pieces.push(Piece::Break { depth });
    }

    /// Wraps `inner` in a layout scope: every [`Rope::push_break`] inside it
    /// indents from the line the scope opens on. This is how a lowering's
    /// generated block structure lines up with the statement it replaces
    /// without any emitter knowing the column it will be printed at.
    ///
    /// A scope is a different boundary from [`Rope::anchored`], which says
    /// which construct owns a stretch of glue so a diagnostic can be traced
    /// back to it. The two coincide for most lowerings, but they answer
    /// different questions, so each emitter that writes breaks opens its
    /// own scope — and [`TargetError::BreakOutsideScope`] catches one that
    /// forgets rather than letting the break fall back to column 0.
    pub(crate) fn scoped(inner: Rope<'a>) -> Rope<'a> {
        let mut out = Rope::new();
        out.pieces.push(Piece::ScopeOpen);
        out.append(inner);
        out.pieces.push(Piece::ScopeClose);
        out
    }

    /// Nests `inner` `depth` indentation units deeper: every break `inner`
    /// wrote in its *own* layout scope moves in by `depth`. Breaks inside a
    /// scope `inner` opened keep their depth — that scope has its own base.
    ///
    /// This is what lets each construct's emitter write its fragment at
    /// depths relative to itself and leave nesting to whoever appends it.
    pub(crate) fn indented(depth: u16, mut inner: Rope<'a>) -> Rope<'a> {
        let mut nested = 0usize;
        for piece in &mut inner.pieces {
            match piece {
                Piece::ScopeOpen => nested += 1,
                Piece::ScopeClose => nested = nested.saturating_sub(1),
                Piece::Break { depth: at } if nested == 0 => *at += depth,
                _ => {}
            }
        }
        inner
    }

    /// Notes that the next thing pushed is the name codegen writes for the
    /// construct at source offset `src`. See [`crate::ScrutineeTemp`].
    pub(crate) fn push_mark(&mut self, src: usize) {
        self.pieces.push(Piece::Mark {
            src,
            kind: MarkKind::Scrutinee,
        });
    }

    /// Notes that the next thing pushed is the receiver expression of the
    /// nested pattern whose tag starts at `src` — the one place a checker
    /// can be asked what that payload's type admits.
    pub(crate) fn push_payload_mark(&mut self, src: usize) {
        self.pieces.push(Piece::Mark {
            src,
            kind: MarkKind::Payload,
        });
    }

    /// Notes that the next copied source byte begins an explicit Result
    /// return value, so a checker query can use its emitted position.
    pub(crate) fn push_result_return_start(&mut self, src: usize) {
        self.pieces.push(Piece::Mark {
            src,
            kind: MarkKind::ResultReturnStart,
        });
    }

    /// Closes the emitted range opened by [`Rope::push_result_return_start`].
    pub(crate) fn push_result_return_end(&mut self, src: usize) {
        self.pieces.push(Piece::Mark {
            src,
            kind: MarkKind::ResultReturnEnd,
        });
    }

    /// Appends `inner` as one construct's glue. `src..src_end` is its
    /// primary display range; `src..owner_end` is the complete syntax node
    /// that owns consequences of this lowering ([`crate::EmitAnchor`]).
    pub(crate) fn anchored(
        &mut self,
        kind: AnchorKind,
        src: usize,
        src_end: usize,
        owner_end: usize,
        inner: Rope<'a>,
    ) {
        self.anchored_with_context(kind, src, src_end, owner_end, None, inner);
    }

    /// [`Rope::anchored`], with a companion source range a diagnostic on
    /// this glue can label ([`EmitAnchor::context`]).
    pub(crate) fn anchored_with_context(
        &mut self,
        kind: AnchorKind,
        src: usize,
        src_end: usize,
        owner_end: usize,
        context: Option<(usize, usize)>,
        inner: Rope<'a>,
    ) {
        self.pieces.push(Piece::Open {
            src,
            src_end,
            owner_end,
            context,
            kind,
        });
        self.append(inner);
        self.pieces.push(Piece::Close);
    }

    pub(crate) fn push_src(&mut self, text: &'a str, src: usize) {
        if !text.is_empty() {
            self.len += text.len();
            self.pieces.push(Piece::Src { text, src });
        }
    }

    /// Inserts `text` immediately before the first source byte at or after
    /// `at` that this rope prints at its top level, or appends it when the
    /// rope prints no such byte.
    ///
    /// The one thing codegen cannot know while emitting is what the
    /// emission will *need* — a pipeline helper's import is decided by the
    /// last pipeline in the file. Appending it was valid (imports hoist)
    /// but read as a stray line at the bottom of the file; this puts it
    /// where a reader looks for an import.
    ///
    /// Only the top level is considered: a piece inside a construct's own
    /// glue belongs to that lowering's arrangement, and a statement cannot
    /// go there anyway. Splitting a pass-through piece in two keeps both
    /// halves pointing at the bytes they always did, so the emission still
    /// covers the source exactly once and still in order.
    pub(crate) fn insert_lit_at_source(&mut self, at: usize, text: impl Into<Cow<'a, str>>) {
        let text = text.into();
        if text.is_empty() {
            return;
        }
        let mut depth = 0usize;
        let mut found: Option<(usize, Option<usize>)> = None;
        for (index, piece) in self.pieces.iter().enumerate() {
            match piece {
                Piece::Open { .. } | Piece::ScopeOpen => depth += 1,
                Piece::Close | Piece::ScopeClose => depth = depth.saturating_sub(1),
                Piece::Src { text, src } if depth == 0 && src + text.len() > at => {
                    found = Some((index, (*src < at).then(|| at - src)));
                    break;
                }
                _ => {}
            }
        }
        let Some((index, split)) = found else {
            self.push_lit(text);
            return;
        };
        self.len += text.len();
        match split {
            None => self.pieces.insert(index, Piece::Lit(text)),
            Some(cut) => {
                let Piece::Src { text: whole, src } = self.pieces[index] else {
                    unreachable!("the piece was matched as a source piece")
                };
                self.pieces[index] = Piece::Src {
                    text: &whole[..cut],
                    src,
                };
                self.pieces.insert(
                    index + 1,
                    Piece::Src {
                        text: &whole[cut..],
                        src: src + cut,
                    },
                );
                self.pieces.insert(index + 1, Piece::Lit(text));
            }
        }
    }

    pub(crate) fn append(&mut self, mut other: Rope<'a>) {
        self.len += other.len;
        self.pieces.append(&mut other.pieces);
    }

    /// The rope's text, when every piece of it is already resolved. A rope
    /// carrying layout breaks answers `None`: its text depends on where it
    /// is printed, and a caller inspecting text is deciding something the
    /// layout must not change.
    pub(crate) fn resolved_text(&self) -> Option<Cow<'_, str>> {
        if self.pieces.iter().any(|piece| piece.is_break()) {
            return None;
        }
        let mut texts = self
            .pieces
            .iter()
            .map(Piece::text)
            .filter(|t| !t.is_empty());
        let first = texts.next().unwrap_or("");
        match texts.next() {
            None => Some(Cow::Borrowed(first)),
            Some(second) => {
                let mut out = String::with_capacity(self.len);
                out.push_str(first);
                out.push_str(second);
                out.extend(texts);
                Some(Cow::Owned(out))
            }
        }
    }

    pub(crate) fn ends_with_newline(&self) -> bool {
        self.pieces
            .iter()
            .rev()
            .find(|piece| !piece.text().is_empty() || matches!(piece, Piece::Break { .. }))
            .is_some_and(Piece::ends_line)
    }

    /// True when the rope's last line carries a `//` line comment — it would
    /// swallow whatever codegen appends on that line. Only the last line is
    /// inspected (pieces are walked back to the nearest newline), so the
    /// check costs a line, not the whole rope.
    pub(crate) fn last_line_has_line_comment(&self) -> bool {
        // `//` can straddle a piece boundary, so the last line is stitched
        // back together before the search — it is one line, not the rope.
        let mut tail: Vec<&str> = Vec::new();
        for piece in self.pieces.iter().rev() {
            if matches!(piece, Piece::Break { .. }) {
                break;
            }
            let text = piece.text();
            match text.rfind('\n') {
                Some(nl) => {
                    tail.push(&text[nl + 1..]);
                    break;
                }
                None => tail.push(text),
            }
        }
        match tail.len() {
            0 => false,
            1 => tail[0].contains("//"),
            _ => {
                let line: String = tail.iter().rev().copied().collect();
                line.contains("//")
            }
        }
    }

    /// Trims whitespace from both ends, exactly like `str::trim` on the
    /// flattened text (Unicode whitespace included). Trimming the front of
    /// a source piece advances its source offset by the removed bytes, so
    /// mappings stay exact.
    ///
    /// Marks carry no text, so they are stepped over rather than trimmed
    /// away — a mark at the edge of a trimmed rope still points at the byte
    /// that ends up there.
    /// The rope with trailing whitespace removed, but with whatever the
    /// source wrote at the front left alone.
    ///
    /// A block arm's body is copied between braces the lowering writes, so
    /// the newline and indentation the author put after their own `{` are
    /// exactly the layout the rest of their block is written against.
    /// Dropping them and opening the block from generated layout puts the
    /// first statement in one column and every following one in another
    /// (TASK-219).
    pub(crate) fn trim_end(mut self) -> Rope<'a> {
        self.trim_back();
        self
    }

    pub(crate) fn trim(mut self) -> Rope<'a> {
        // front
        let mut front = 0;
        while let Some(first) = self.pieces.get_mut(front) {
            if matches!(first, Piece::Break { .. }) {
                self.pieces.remove(front);
                continue;
            }
            if first.text().is_empty() && !first.is_text() {
                front += 1;
                continue;
            }
            let text = first.text();
            let trimmed = text.trim_start();
            if trimmed.is_empty() {
                self.len -= text.len();
                self.pieces.remove(front);
                continue;
            }
            let cut = text.len() - trimmed.len();
            if cut > 0 {
                self.len -= cut;
                first.cut_front(cut);
            }
            break;
        }
        self.trim_back();
        self
    }

    fn trim_back(&mut self) {
        let mut back = self.pieces.len();
        while back > 0 {
            let last = &mut self.pieces[back - 1];
            if matches!(last, Piece::Break { .. }) {
                self.pieces.remove(back - 1);
                back -= 1;
                continue;
            }
            if last.text().is_empty() && !last.is_text() {
                back -= 1;
                continue;
            }
            let text = last.text();
            let trimmed = text.trim_end();
            if trimmed.is_empty() {
                self.len -= text.len();
                self.pieces.remove(back - 1);
                back -= 1;
                continue;
            }
            self.len -= text.len() - trimmed.len();
            let keep = trimmed.len();
            last.truncate(keep);
            break;
        }
    }

    /// Builds, validates, and prints the source-preserving target.
    ///
    /// Both validators run in every build: a violated target contract is an
    /// internal compiler error, and a release build must fail on it exactly
    /// like a debug build so a wrong lowering is never shipped silently
    /// (`docs/design/program-lowering.md` §11).
    pub(crate) fn flatten(self, source: &'a str, preservation: &SourcePreservation) -> Flat {
        let mut target = TargetFile::from_rope(self, source.len());
        target.source = Some(source);
        if let Err(error) = target.validate() {
            error.into_ice().raise();
        }
        if let Err(error) = target.validate_source_preservation(preservation) {
            error.raise();
        }
        target.print()
    }
}

/// A flattened rope: the text, and everything language tooling reads off
/// the emission.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Flat {
    pub code: String,
    pub mappings: Vec<EmitMapping>,
    pub scrutinee_temps: Vec<ScrutineeTemp>,
    pub payload_temps: Vec<PayloadTemp>,
    pub anchors: Vec<EmitAnchor>,
    /// Explicit Result return values in source and emitted coordinates.
    pub result_return_temps: Vec<ResultReturnTemp>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_assigns_exact_construct_and_file_origins() {
        let mut rope = Rope::new();
        rope.push_src("const ", 0);
        let mut generated = Rope::new();
        generated.push_lit("value");
        rope.anchored(AnchorKind::Match, 6, 11, 11, generated);
        rope.push_lit(";\n");

        let target = TargetFile::from_rope(rope, 11);
        assert_eq!(target.validate(), Ok(()));
        assert!(matches!(
            target.pieces[0],
            TargetPiece::Source {
                origin: ExactOrigin { start: 0, end: 6 },
                ..
            }
        ));
        assert!(matches!(
            target.pieces[2],
            TargetPiece::Generated {
                origin: SourceOrigin::Construct {
                    kind: AnchorKind::Match,
                    ..
                },
                ..
            }
        ));
        assert!(matches!(
            target.pieces[4],
            TargetPiece::Generated {
                origin: SourceOrigin::Synthetic {
                    reason: SyntheticReason::UnanchoredGenerated,
                    ..
                },
                ..
            }
        ));
        let flat = target.print();
        assert_eq!(flat.code, "const value;\n");
        assert_eq!(
            flat.mappings,
            [EmitMapping {
                src: 0,
                out: 0,
                len: 6
            }]
        );
    }

    #[test]
    fn target_rejects_a_source_piece_outside_the_input() {
        let mut rope = Rope::new();
        rope.push_src("x", 2);
        let target = TargetFile::from_rope(rope, 2);
        assert_eq!(
            target.validate(),
            Err(TargetError::SourceOutOfBounds { start: 2, end: 3 })
        );
    }

    #[test]
    fn target_rejects_a_break_with_no_layout_scope() {
        // A break's indentation is meaningless without a scope to measure
        // it from, so the target refuses one — an emitter that writes block
        // structure has to say where that structure starts.
        let mut loose = Rope::new();
        loose.push_lit("x");
        loose.push_break(1);
        let target = TargetFile::from_rope(loose, 0);
        assert_eq!(target.validate(), Err(TargetError::BreakOutsideScope));

        let mut scoped = Rope::new();
        scoped.push_lit("x");
        scoped.push_break(1);
        let target = TargetFile::from_rope(Rope::scoped(scoped), 0);
        assert_eq!(target.validate(), Ok(()));
    }

    #[test]
    fn target_rejects_unbalanced_anchor_structure() {
        let rope = Rope {
            pieces: vec![Piece::Close],
            len: 0,
        };
        let target = TargetFile::from_rope(rope, 0);
        assert_eq!(target.validate(), Err(TargetError::CloseWithoutOpen));
    }

    fn preserved<'a>(source: &'a str, rope: Rope<'a>) -> TargetFile<'a> {
        let mut target = TargetFile::from_rope(rope, source.len());
        target.source = Some(source);
        target
    }

    fn owned_whole(source: &str) -> SourcePreservation {
        SourcePreservation {
            owned: vec![SourceSpan {
                start: 0,
                end: source.len(),
            }],
            relocated: Vec::new(),
            rewritten: Vec::new(),
        }
    }

    #[test]
    fn preservation_accepts_a_faithful_pass_through() {
        let source = "const a = 1;";
        let mut rope = Rope::new();
        rope.push_src(&source[..6], 0);
        rope.push_src(&source[6..], 6);
        let target = preserved(source, rope);
        assert_eq!(
            target.validate_source_preservation(&owned_whole(source)),
            Ok(())
        );
    }

    #[test]
    fn preservation_rejects_a_pass_through_byte_printed_twice() {
        // The defect shape: a relocated range whose bytes also stay in
        // place — the relocation excuses the order, never the count.
        let source = "const a = 1;";
        let mut rope = Rope::new();
        rope.push_src(source, 0);
        rope.push_src(&source[6..7], 6);
        let target = preserved(source, rope);
        let preservation = SourcePreservation {
            owned: vec![SourceSpan {
                start: 0,
                end: source.len(),
            }],
            relocated: vec![SourceSpan { start: 6, end: 7 }],
            rewritten: Vec::new(),
        };
        let error = target
            .validate_source_preservation(&preservation)
            .expect_err("duplicate must be rejected");
        assert_eq!(error.invariant, Invariant::SourceEmittedTwice);
        assert_eq!(error.stage, LoweringStage::TargetSourcePreservation);
    }

    #[test]
    fn preservation_rejects_a_dropped_pass_through_byte() {
        let source = "const a = 1;";
        let mut rope = Rope::new();
        rope.push_src(&source[..6], 0);
        // bytes 6.. never emitted
        let target = preserved(source, rope);
        let error = target
            .validate_source_preservation(&owned_whole(source))
            .expect_err("drop must be rejected");
        assert_eq!(error.invariant, Invariant::SourceOmitted);
        assert_eq!(error.span, Some(SourceSpan { start: 6, end: 7 }));
    }

    #[test]
    fn preservation_allows_dropped_whitespace_only() {
        let source = "a  b";
        let mut rope = Rope::new();
        rope.push_src(&source[..1], 0);
        rope.push_src(&source[3..], 3);
        let target = preserved(source, rope);
        assert_eq!(
            target.validate_source_preservation(&owned_whole(source)),
            Ok(())
        );
    }

    #[test]
    fn preservation_rejects_an_unregistered_reorder() {
        let source = "ab";
        let mut rope = Rope::new();
        rope.push_src(&source[1..], 1);
        rope.push_src(&source[..1], 0);
        let target = preserved(source, rope);
        let error = target
            .validate_source_preservation(&owned_whole(source))
            .expect_err("reorder must be rejected");
        assert_eq!(error.invariant, Invariant::SourceReordered);
    }

    #[test]
    fn preservation_accepts_a_registered_relocation() {
        let source = "ab";
        let mut rope = Rope::new();
        rope.push_src(&source[1..], 1);
        rope.push_src(&source[..1], 0);
        let target = preserved(source, rope);
        let preservation = SourcePreservation {
            owned: vec![SourceSpan { start: 0, end: 2 }],
            relocated: vec![SourceSpan { start: 1, end: 2 }],
            rewritten: Vec::new(),
        };
        assert_eq!(target.validate_source_preservation(&preservation), Ok(()));
    }

    #[test]
    fn preservation_exempts_a_registered_rewrite_from_coverage() {
        let source = "return x;";
        let mut rope = Rope::new();
        // The exit rewrite prints only the argument; the frame is claimed.
        rope.push_src(&source[7..8], 7);
        let target = preserved(source, rope);
        let preservation = SourcePreservation {
            owned: vec![SourceSpan { start: 0, end: 9 }],
            relocated: Vec::new(),
            rewritten: vec![
                SourceSpan { start: 0, end: 7 },
                SourceSpan { start: 8, end: 9 },
            ],
        };
        assert_eq!(target.validate_source_preservation(&preservation), Ok(()));
    }
}
