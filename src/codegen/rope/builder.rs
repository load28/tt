//! Mapping-aware rope construction and final flattening.

use super::*;

#[derive(Default)]
pub(crate) struct Rope<'a> {
    pub(super) pieces: std::collections::VecDeque<Piece<'a>>,
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
            self.pieces.push_back(Piece::Lit(text));
        }
    }

    /// Ends the current line of generated glue and opens the next one
    /// `depth` indentation units inside the enclosing layout scope. The
    /// whitespace itself is resolved when the target is printed
    /// ([`Rope::scoped`]), because it depends on where the scope opened.
    pub(crate) fn push_break(&mut self, depth: u16) {
        self.pieces.push_back(Piece::Break { depth });
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
    pub(crate) fn push_scope_open(&mut self) {
        self.pieces.push_back(Piece::ScopeOpen);
    }

    pub(crate) fn push_scope_close(&mut self) {
        self.pieces.push_back(Piece::ScopeClose);
    }

    pub(crate) fn scoped(inner: Rope<'a>) -> Rope<'a> {
        let mut out = Rope::new();
        out.pieces.push_back(Piece::ScopeOpen);
        out.append(inner);
        out.pieces.push_back(Piece::ScopeClose);
        out
    }

    pub(crate) fn braced(inner: Rope<'a>) -> Rope<'a> {
        let mut block = Rope::new();
        block.push_lit("{");
        block.push_break(1);
        block.append(Rope::indented(1, inner.trim()));
        block.push_break(0);
        block.push_lit("}");
        Rope::scoped(block)
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
        self.pieces.push_back(Piece::Mark {
            src,
            kind: MarkKind::Scrutinee,
        });
    }

    /// Notes that the glue pushed next is written for the source construct
    /// part at `src` ([`MarkKind::SourcePoint`]).
    pub(crate) fn push_source_point(&mut self, src: usize) {
        self.pieces.push_back(Piece::Mark {
            src,
            kind: MarkKind::SourcePoint,
        });
    }

    /// Notes that the next thing pushed is the receiver expression of the
    /// nested pattern whose tag starts at `src` — the one place a checker
    /// can be asked what that payload's type admits.
    pub(crate) fn push_payload_mark(&mut self, src: usize) {
        self.pieces.push_back(Piece::Mark {
            src,
            kind: MarkKind::Payload,
        });
    }

    /// Writes the tag literal the receiver of the nested pattern at `src` is
    /// compared with, marked as that payload's ([`crate::PayloadTemp::tag`]).
    pub(crate) fn push_payload_tag(&mut self, src: usize, literal: String) {
        self.pieces.push_back(Piece::Mark {
            src,
            kind: MarkKind::PayloadTagStart,
        });
        self.push_lit(literal);
        self.pieces.push_back(Piece::Mark {
            src,
            kind: MarkKind::PayloadTagEnd,
        });
    }

    /// Declares storage whose expected type is supplied by its contextual host.
    pub(crate) fn push_value_declaration(&mut self, name: &str) {
        self.push_lit(format!("let {name}"));
        self.pieces.push_back(Piece::Mark {
            src: 0,
            kind: MarkKind::ContextualSlot,
        });
        self.push_lit(";");
    }

    /// Declares storage for the index of the arm a dispatch selects.
    pub(crate) fn push_selector_declaration(&mut self, name: &str) {
        self.push_lit(format!("let {name}"));
        self.pieces.push_back(Piece::Mark {
            src: 0,
            kind: MarkKind::SelectorSlot,
        });
        self.push_lit(";");
    }

    pub(crate) fn push_asserted_declaration(&mut self, name: &str, annotation: String) {
        self.push_lit(format!("let {name}"));
        self.pieces.push_back(Piece::Mark {
            src: 0,
            kind: MarkKind::ContextualSlot,
        });
        self.push_lit(annotation);
        self.pieces.push_back(Piece::Mark {
            src: 0,
            kind: MarkKind::AssertedAnnotationEnd,
        });
        self.push_lit(";");
    }

    pub(crate) fn push_operand_declaration(&mut self, name: &str) {
        self.push_lit(format!("let {name}"));
        self.pieces.push_back(Piece::Mark {
            src: 0,
            kind: MarkKind::OperandSlot,
        });
        self.push_lit(";");
    }

    pub(crate) fn push_value_definition(&mut self, name: &str) {
        self.push_lit(format!("const {name}"));
        self.pieces.push_back(Piece::Mark {
            src: 0,
            kind: MarkKind::ContextualSlot,
        });
        self.push_lit(" = ");
    }

    /// Starts a captured value while retaining its contextual annotation site.
    pub(crate) fn push_value_capture(&mut self, name: &str) {
        self.push_lit(format!("const {name}"));
        self.pieces.push_back(Piece::Mark {
            src: 0,
            kind: MarkKind::ContextualSlot,
        });
        self.push_lit(" = (");
    }

    /// Notes that the next copied source byte begins an explicit Result
    /// return value, so a checker query can use its emitted position.
    pub(crate) fn push_result_return_start(&mut self, src: usize) {
        self.pieces.push_back(Piece::Mark {
            src,
            kind: MarkKind::ResultReturnStart,
        });
    }

    /// Closes the emitted range opened by [`Rope::push_result_return_start`].
    pub(crate) fn push_result_return_end(&mut self, src: usize) {
        self.pieces.push_back(Piece::Mark {
            src,
            kind: MarkKind::ResultReturnEnd,
        });
    }

    pub(crate) fn push_declared_name(
        &mut self,
        text: impl Into<Cow<'a, str>>,
        src: usize,
        src_end: usize,
    ) {
        self.pieces.push_back(Piece::Mark {
            src,
            kind: MarkKind::DeclaredNameStart,
        });
        self.push_lit(text);
        self.pieces.push_back(Piece::Mark {
            src: src_end,
            kind: MarkKind::DeclaredNameEnd,
        });
    }

    pub(crate) fn push_relocated_operand(
        &mut self,
        text: impl Into<Cow<'a, str>>,
        src: usize,
        src_end: usize,
    ) {
        let mut read = Rope::new();
        read.push_lit(text);
        self.relocated_operand(src, src_end, read);
    }

    pub(crate) fn relocated_operand(&mut self, src: usize, src_end: usize, read: Rope<'a>) {
        self.pieces.push_back(Piece::Mark {
            src,
            kind: MarkKind::RelocatedOperandStart,
        });
        self.append(read);
        self.pieces.push_back(Piece::Mark {
            src: src_end,
            kind: MarkKind::RelocatedOperandEnd,
        });
    }

    pub(crate) fn push_destructured_list_start(&mut self, src: usize) {
        self.pieces.push_back(Piece::Mark {
            src,
            kind: MarkKind::DestructuredListStart,
        });
    }

    pub(crate) fn push_destructured_list_end(&mut self, src_end: usize) {
        self.pieces.push_back(Piece::Mark {
            src: src_end,
            kind: MarkKind::DestructuredListEnd,
        });
    }

    pub(crate) fn push_shared_binding(
        &mut self,
        text: impl Into<Cow<'a, str>>,
        occurrences: &[BindingOccurrence],
    ) {
        self.pieces.push_back(Piece::Mark {
            src: occurrences.first().map_or(0, |occurrence| occurrence.src),
            kind: MarkKind::SharedBindingStart,
        });
        for occurrence in occurrences {
            self.pieces.push_back(Piece::Mark {
                src: occurrence.src,
                kind: MarkKind::SharedBindingOccurrence {
                    end: occurrence.src_end,
                    shorthand: occurrence.shorthand,
                    declared: occurrence.declared,
                },
            });
        }
        self.push_lit(text);
        self.pieces.push_back(Piece::Mark {
            src: occurrences
                .last()
                .map_or(0, |occurrence| occurrence.src_end),
            kind: MarkKind::SharedBindingEnd,
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
        self.pieces.push_back(Piece::Open {
            src,
            src_end,
            owner_end,
            context,
            kind,
        });
        self.append(inner);
        self.pieces.push_back(Piece::Close);
    }

    pub(crate) fn push_restatement(&mut self, text: impl Into<Cow<'a, str>>) {
        self.pieces.push_back(Piece::Mark {
            src: 0,
            kind: MarkKind::RestatementStart,
        });
        self.push_lit(text);
        self.pieces.push_back(Piece::Mark {
            src: 0,
            kind: MarkKind::RestatementEnd,
        });
    }

    pub(crate) fn push_src(&mut self, text: &'a str, src: usize) {
        if !text.is_empty() {
            self.len += text.len();
            self.pieces.push_back(Piece::Src { text, src });
        }
    }

    /// Inserts `declarations` after the leading top-level source pieces that
    /// print only bytes before `at`, splitting the piece that straddles
    /// `at`, so they precede every piece written for source at or after
    /// `at`, generated glue included. Each declaration is its own
    /// [`crate::InsertedGlue`]: text inserted between two of them stands
    /// at `at`, text inserted inside one changes it.
    ///
    /// The one thing codegen cannot know while emitting is what the
    /// emission will *need* — a pipeline helper's import is decided by the
    /// last pipeline in the file. Appending it was valid (imports hoist)
    /// but read as a stray line at the bottom of the file; this puts it
    /// where a reader looks for an import.
    ///
    /// Splitting a pass-through piece in two keeps both halves pointing at
    /// the bytes they always did, so the emission still covers the source
    /// exactly once and still in order.
    pub(crate) fn insert_declarations_at_source<T: Into<Cow<'a, str>>>(
        &mut self,
        at: usize,
        declarations: impl IntoIterator<Item = T>,
    ) {
        let declarations: Vec<Cow<'a, str>> = declarations
            .into_iter()
            .map(Into::into)
            .filter(|text| !text.is_empty())
            .collect();
        if declarations.is_empty() {
            return;
        }
        let mut index = 0;
        let mut split = None;
        while let Some(Piece::Src { text, src }) = self.pieces.get(index) {
            if src + text.len() <= at {
                index += 1;
                continue;
            }
            if *src < at {
                split = Some(at - src);
            }
            break;
        }
        self.len += declarations.iter().map(|text| text.len()).sum::<usize>();
        let inserted: Vec<Piece<'a>> = declarations
            .into_iter()
            .flat_map(|text| {
                [
                    Piece::Mark {
                        src: at,
                        kind: MarkKind::InsertedStart,
                    },
                    Piece::Lit(text),
                    Piece::Mark {
                        src: at,
                        kind: MarkKind::InsertedEnd,
                    },
                ]
            })
            .collect();
        match split {
            None => insert_run(&mut self.pieces, index, inserted),
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
                insert_run(&mut self.pieces, index + 1, inserted);
            }
        }
    }

    /// Appends `other`. The shorter of the two moves, so a rope built by
    /// wrapping its children level by level (a nested template, a chain of
    /// blocks) moves each piece a logarithmic number of times rather than
    /// once per level.
    pub(crate) fn append(&mut self, mut other: Rope<'a>) {
        self.len += other.len;
        if self.pieces.len() < other.pieces.len() {
            std::mem::swap(&mut self.pieces, &mut other.pieces);
            while let Some(piece) = other.pieces.pop_back() {
                self.pieces.push_front(piece);
            }
        } else {
            self.pieces.append(&mut other.pieces);
        }
    }

    /// The rope's text, when every piece of it is already resolved. A rope
    /// carrying layout breaks answers `None`: its text depends on where it
    /// is printed, and a caller inspecting text is deciding something the
    /// layout must not change.
    pub(crate) fn resolved_text(&self) -> Option<Cow<'_, str>> {
        if !self.is_resolved() {
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

    /// Where the rope's text starts in the source, when its first text is
    /// copied from the source, and where the last byte it copies ends.
    pub(crate) fn source_bounds(&self) -> Option<(usize, usize)> {
        let Piece::Src { src: start, .. } = self
            .pieces
            .iter()
            .find(|piece| piece.is_text() && !piece.text().is_empty())?
        else {
            return None;
        };
        let end = self
            .pieces
            .iter()
            .filter_map(|piece| match piece {
                Piece::Src { text, src } => Some(src + text.len()),
                _ => None,
            })
            .max()?;
        Some((*start, end))
    }

    pub(crate) fn source_edges(&self) -> Option<(usize, usize)> {
        let text = |piece: &&Piece<'a>| piece.is_text() && !piece.text().is_empty();
        let Piece::Src { src: first, .. } = self.pieces.iter().find(text)? else {
            return None;
        };
        let Piece::Src { text: last, src } = self.pieces.iter().rev().find(text)? else {
            return None;
        };
        Some((*first, src + last.len()))
    }

    pub(crate) fn is_resolved(&self) -> bool {
        !self.pieces.iter().any(|piece| piece.is_break())
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
    pub(crate) fn last_line_has_line_comment(&self, source_kind: SourceKind) -> bool {
        // Piece boundaries carry provenance, not lexical meaning. Rebuild the
        // text with abstract breaks represented as newlines, then let the
        // language lexer identify the final significant token. Only trailing
        // trivia after that token can be a line comment: `//` inside a string,
        // template, regex, or JSX raw text remains part of its token.
        let mut text = String::with_capacity(self.len + 8);
        for piece in &self.pieces {
            if matches!(piece, Piece::Break { .. }) {
                text.push('\n');
            } else {
                text.push_str(piece.text());
            }
        }
        if !text.contains("//") {
            return false;
        }
        let tokens = crate::lexer::lex_with_kind(&text, 0, text.len(), source_kind);
        let mut at = tokens.last().map_or(0, |token| token.span.end);
        let bytes = text.as_bytes();
        while at < bytes.len() {
            while at < bytes.len() && crate::scanner::is_ws(bytes[at]) {
                at += 1;
            }
            if at >= bytes.len() {
                return false;
            }
            if bytes[at..].starts_with(b"//") {
                return crate::scanner::line_end(bytes, at, bytes.len()) == bytes.len();
            }
            if bytes[at..].starts_with(b"/*") {
                let Some(close) = crate::scanner::find_subslice(bytes, b"*/", at + 2, bytes.len())
                else {
                    return false;
                };
                at = close + 2;
                continue;
            }
            return false;
        }
        false
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
    /// (`docs/design/program-lowering.md` §11). `boundaries` are the sorted
    /// starts of the source statements an automatic semicolon separates
    /// from the statement before them, which the target keeps separate.
    pub(crate) fn flatten(
        self,
        source: &'a str,
        source_kind: SourceKind,
        boundaries: &[usize],
        preservation: &SourcePreservation,
        governed: &[super::GovernedStatement],
        line_comment_ends: Vec<usize>,
    ) -> Flat {
        let mut target = TargetFile::from_rope(self, source.len());
        target.source = Some(source);
        target.line_comment_ends = line_comment_ends;
        target.separate_statements(boundaries, source_kind);
        target.separate_tokens();
        if let Err(error) = target.validate() {
            error.into_ice().raise();
        }
        if let Err(error) = target.validate_source_preservation(preservation) {
            error.raise();
        }
        target.print(crate::line_ending(source), governed)
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
    pub contextual_slots: Vec<usize>,
    pub selector_slots: Vec<usize>,
    pub operand_slots: Vec<usize>,
    pub asserted_slots: Vec<(usize, usize)>,
    pub restatements: Vec<(usize, usize)>,
    pub generated_names: std::collections::HashSet<String>,
    pub declared_names: Vec<DeclaredName>,
    pub shared_bindings: Vec<SharedBinding>,
    pub destructured_lists: Vec<crate::DestructuredList>,
    pub relocated_operands: Vec<crate::RelocatedOperand>,
    pub inserted: Vec<crate::InsertedGlue>,
    pub support_imports: Vec<crate::StdModule>,
    pub commonjs: bool,
    pub single_line_breaks: Vec<usize>,
}

/// Inserts `run` before position `at`, moving whichever side of `at` is
/// shorter.
fn insert_run<'a>(
    pieces: &mut std::collections::VecDeque<Piece<'a>>,
    at: usize,
    run: Vec<Piece<'a>>,
) {
    if at <= pieces.len() / 2 {
        let head: Vec<Piece<'a>> = pieces.drain(..at).collect();
        for piece in run.into_iter().rev().chain(head.into_iter().rev()) {
            pieces.push_front(piece);
        }
    } else {
        let tail = pieces.split_off(at);
        pieces.extend(run);
        pieces.extend(tail);
    }
}
