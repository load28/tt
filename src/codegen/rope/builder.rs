//! Mapping-aware rope construction and final flattening.

use super::*;

#[derive(Default)]
pub(crate) struct Rope<'a> {
    pub(super) pieces: Vec<Piece<'a>>,
    /// Total byte length of the pieces — [`Rope::flatten`]'s exact capacity.
    pub(super) len: usize,
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
