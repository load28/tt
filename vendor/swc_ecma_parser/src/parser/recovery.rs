//! Grammar-owned recovery for editor syntax trees. Strict parsing is unchanged.

use super::*;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryMode {
    #[default]
    Strict,
    Editor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryContext {
    Statement,
    Expression,
    Delimiter,
    Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryKind {
    MissingToken,
    MissingExpression,
    SkippedInput,
    MissingType,
}

#[derive(Debug, Clone)]
pub struct RecoveryRecord {
    pub id: RecoveryId,
    pub kind: RecoveryKind,
    pub context: RecoveryContext,
    pub span: Span,
    pub owner: Span,
    pub expected: String,
    /// Materialize only syntax whose absence would capture a later statement.
    pub replacement: Option<&'static str>,
}

#[derive(Debug, Default, Clone)]
pub(super) struct RecoveryState {
    mode: RecoveryMode,
    records: Vec<RecoveryRecord>,
}

/// Records are only appended during a speculation, so its rollback is the
/// record count when it began.
#[derive(Debug, Clone, Copy)]
pub(super) struct RecoveryCheckpoint {
    records: usize,
}

impl RecoveryState {
    pub(super) fn checkpoint(&self) -> RecoveryCheckpoint {
        RecoveryCheckpoint {
            records: self.records.len(),
        }
    }

    pub(super) fn rollback(&mut self, checkpoint: RecoveryCheckpoint) {
        self.records.truncate(checkpoint.records);
    }
}

impl<I: Tokens> Parser<I> {
    pub(super) fn recover_type(&mut self, start: BytePos, error: Error) -> PResult<Box<TsType>> {
        self.emit_error(error);
        // '=' starts the initializer, not part of the missing annotation.
        // Other list/statement terminators belong to their enclosing grammar.
        self.skip_to_recovery_boundary(true);
        let end = self.cur_pos().max(start);
        if self.input().prev_span().hi < start {
            self.input_mut().prev_span = Span::new_with_checked(start, start);
        }
        self.record_recovery(
            start,
            RecoveryContext::Type,
            RecoveryKind::MissingType,
            "type".into(),
        );
        // This node is a syntax-only placeholder. The source-preserving emitter
        // retains the user's annotation for TypeScript's own error recovery.
        Ok(Box::new(TsType::TsKeywordType(TsKeywordType {
            span: Span::new_with_checked(start, end),
            kind: TsKeywordTypeKind::TsUnknownKeyword,
        })))
    }
    pub(super) fn materialize_last_recovery(&mut self, text: &'static str) {
        self.recovery
            .records
            .last_mut()
            .expect("a recovery was just recorded")
            .replacement = Some(text);
    }

    pub fn set_recovery_mode(&mut self, mode: RecoveryMode) {
        self.recovery.mode = mode;
    }

    pub fn take_recoveries(&mut self) -> Vec<RecoveryRecord> {
        std::mem::take(&mut self.recovery.records)
    }

    pub(super) fn editor_recovery(&self) -> bool {
        self.recovery.mode == RecoveryMode::Editor && !self.ctx().contains(Context::IgnoreError)
    }

    fn record_recovery(
        &mut self,
        start: BytePos,
        context: RecoveryContext,
        kind: RecoveryKind,
        expected: String,
    ) {
        let end = self.cur_pos().max(start);
        let span = Span::new_with_checked(start, end);
        self.recovery.records.push(RecoveryRecord {
            id: RecoveryId(self.recovery.records.len() as u32),
            kind,
            context,
            span,
            owner: span,
            expected,
            replacement: None,
        });
    }

    /// These tokens cannot begin an assignment expression. They belong to the
    /// containing statement/list; retaining them lets that production resume.
    fn is_expression_boundary(token: Token) -> bool {
        matches!(
            token,
            Token::Semi
                | Token::Comma
                | Token::RParen
                | Token::RBracket
                | Token::RBrace
                | Token::Eof
                | Token::Const
                | Token::Let
                | Token::Var
                | Token::Export
                | Token::Return
                | Token::Throw
                | Token::Else
        )
    }

    fn at_expression_boundary(&mut self) -> bool {
        let token = self.input().cur();
        if token == Token::Let {
            return matches!(
                self.input_mut().peek(),
                Some(Token::Ident | Token::LBrace | Token::LBracket)
            );
        }
        Self::is_expression_boundary(token)
    }

    pub(super) fn ends_recovery_object(&mut self) -> bool {
        if !self.ends_recovery_list(Token::RBrace) {
            return false;
        }
        // Keywords are valid property/method names, even when the keyword
        // starts a statement in another grammatical context.
        if matches!(
            self.input().cur(),
            Token::Const
                | Token::Let
                | Token::Var
                | Token::Return
                | Token::Throw
                | Token::Export
                | Token::Else
        ) {
            return !matches!(
                self.input_mut().peek(),
                Some(
                    Token::Colon
                        | Token::LParen
                        | Token::Comma
                        | Token::RBrace
                        | Token::QuestionMark
                        | Token::Lt
                )
            );
        }
        true
    }

    /// TypeScript's `parseRightSideOfDot`: after a line break, an identifier
    /// or keyword followed on the same line by another one starts a new
    /// construct, so the name after `.` is missing. Any other token is the
    /// member name (`obj.\nconst\nx` reads `obj.const`).
    pub(super) fn missing_member_name(&mut self) -> Option<IdentName> {
        if !self.editor_recovery()
            || !self.input().had_line_break_before_cur()
            || !self.input().cur().is_word()
        {
            return None;
        }
        if !self.input_mut().peek().is_some_and(|next| next.is_word())
            || self.input_mut().has_linebreak_between_cur_and_peeked()
        {
            return None;
        }
        let at = self.input().prev_span().hi;
        let span = Span::new_with_checked(at, at);
        self.emit_err(span, SyntaxError::ExpectedIdent);
        // The missing name is immediately after '.', before any intervening
        // trivia or the token owned by the containing statement list.
        self.recovery.records.push(RecoveryRecord {
            id: RecoveryId(self.recovery.records.len() as u32),
            kind: RecoveryKind::MissingToken,
            context: RecoveryContext::Expression,
            span,
            owner: span,
            expected: "identifier".into(),
            replacement: None,
        });
        Some(IdentName::new("".into(), span))
    }

    pub(super) fn ends_recovery_list(&mut self, closing: Token) -> bool {
        let token = self.input().cur();
        token != closing && self.at_expression_boundary() && token != Token::Comma
    }

    pub(super) fn can_recover_missing_token(&mut self, expected: Token) -> bool {
        self.editor_recovery()
            && matches!(
                expected,
                Token::RParen | Token::RBracket | Token::RBrace | Token::Gt
            )
            && self.at_expression_boundary()
    }

    pub(super) fn recover_missing_token(&mut self, expected: Token) -> bool {
        if !self.can_recover_missing_token(expected) {
            return false;
        }
        let at = self.cur_pos();
        let message = match expected {
            Token::RParen => ")",
            Token::RBracket => "]",
            Token::RBrace => "}",
            Token::Gt => ">",
            _ => unreachable!(),
        }
        .to_string();
        self.record_recovery(
            at,
            RecoveryContext::Delimiter,
            RecoveryKind::MissingToken,
            message.clone(),
        );
        self.recovery.records.last_mut().unwrap().replacement = Some(match expected {
            Token::RParen => ")",
            Token::RBracket => "]",
            Token::RBrace => "}",
            Token::Gt => ">",
            _ => unreachable!(),
        });
        let got = self.input_mut().dump_cur();
        self.emit_err(
            Span::new_with_checked(at, at),
            SyntaxError::Expected(message, got),
        );
        true
    }

    pub(super) fn recover_expression(
        &mut self,
        start: BytePos,
        error: Error,
    ) -> PResult<Box<Expr>> {
        // A failed regexp scan leaves the lexer in regexp-rescan mode. The
        // scanner has consumed its lexical region; continuing must scan the
        // next token, not retry the same unterminated regexp forever.
        self.input_mut().set_next_regexp(None);
        self.emit_error(error);
        self.skip_to_expression_boundary();
        let missing = start == self.cur_pos();
        if missing && self.input().prev_span().hi < start {
            // A missing node occupies a grammar slot but consumes no token.
            // SWC finishes enclosing nodes at prev_span.hi; include this
            // virtual slot so their spans cannot end before their start.
            self.input_mut().prev_span = Span::new_with_checked(start, start);
        }
        self.record_recovery(
            start,
            RecoveryContext::Expression,
            if missing {
                RecoveryKind::MissingExpression
            } else {
                RecoveryKind::SkippedInput
            },
            "expression".into(),
        );
        if missing
            && matches!(
                self.input().cur(),
                Token::Const
                    | Token::Let
                    | Token::Var
                    | Token::Export
                    | Token::Return
                    | Token::Throw
            )
        {
            self.recovery.records.last_mut().unwrap().replacement = Some("(undefined as any)");
        }
        Ok(Box::new(Expr::Invalid(Invalid {
            span: Span::new_with_checked(start, self.cur_pos().max(start)),
        })))
    }

    fn skip_to_expression_boundary(&mut self) {
        self.skip_to_recovery_boundary(false);
    }

    fn skip_to_recovery_boundary(&mut self, stop_at_initializer: bool) {
        let mut delimiters = Vec::new();
        loop {
            let token = self.input().cur();
            if token == Token::Eof
                || delimiters.is_empty()
                    && (self.at_expression_boundary() || stop_at_initializer && token == Token::Eq)
            {
                return;
            }
            match token {
                // Template heads include `${`. Their matching `}` must be
                // rescanned in template mode, so text and nested interpolations
                // cannot become statements or swallow the following source.
                Token::TemplateHead => delimiters.push(Token::TemplateTail),
                Token::RBrace if delimiters.last() == Some(&Token::TemplateTail) => {
                    self.input_mut().rescan_template_token(false);
                    if self.input().cur() != Token::TemplateMiddle {
                        delimiters.pop();
                    }
                }
                Token::LParen => delimiters.push(Token::RParen),
                Token::LBracket => delimiters.push(Token::RBracket),
                Token::LBrace => delimiters.push(Token::RBrace),
                Token::RParen | Token::RBracket | Token::RBrace => {
                    if delimiters.last() != Some(&token) {
                        return;
                    }
                    delimiters.pop();
                }
                _ => {}
            }
            self.bump();
        }
    }

    pub(super) fn parse_recoverable_statement<T: From<Stmt>>(
        &mut self,
        parse_module_decl: &impl Fn(&mut Self, Vec<Decorator>) -> PResult<T>,
    ) -> PResult<T> {
        if !self.editor_recovery() {
            return self.parse_stmt_like(true, parse_module_decl);
        }
        let start = self.cur_pos();
        let first_record = self.recovery.records.len();
        let result = self.parse_stmt_like(true, parse_module_decl);
        let result = match result {
            Err(error) if self.editor_recovery() => {
                self.emit_error(error);
                // The statement is skipped whole: no node of it remains, so
                // neither does what its productions recorded.
                self.recovery.records.truncate(first_record);
                // A failed production that consumed no input cannot be retried
                // at the same token by its enclosing statement list.
                if self.cur_pos() == start && self.input().cur() != Token::Eof {
                    self.bump();
                }
                self.skip_to_expression_boundary();
                if self.input().cur() == Token::Semi {
                    self.bump();
                }
                self.record_recovery(
                    start,
                    RecoveryContext::Statement,
                    RecoveryKind::SkippedInput,
                    "statement".into(),
                );
                Ok(T::from(Stmt::Expr(ExprStmt {
                    span: Span::new_with_checked(start, self.input().prev_span().hi.max(start)),
                    expr: Box::new(Expr::Invalid(Invalid {
                        span: Span::new_with_checked(start, self.input().prev_span().hi.max(start)),
                    })),
                })))
            }
            result => result,
        };
        // A stray enclosing delimiter at source level has no production to
        // return to. Even a recovered expression statement may consume zero
        // tokens there; the statement list must make progress.
        if self.cur_pos() == start && self.input().cur() != Token::Eof {
            self.bump();
        }
        let owner = Span::new_with_checked(start, self.input().prev_span().hi.max(start));
        for record in &mut self.recovery.records[first_record..] {
            // Inner statements already have a more precise owner.
            if record.owner == record.span {
                record.owner = owner;
            }
        }
        result
    }
}
