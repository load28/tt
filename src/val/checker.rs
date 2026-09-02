//! Lexical-scope walk and mutation/call probe collection.

use super::*;

/// The `val` checker's walk state.
pub(super) struct Checker<'a> {
    pub(super) src: &'a str,
    pub(super) signatures: &'a HashMap<&'a str, Option<Vec<ParamSig>>>,
    /// Probe mode: collect method calls instead of reporting violations.
    /// The violations are the same either way — the file has already been
    /// checked by the time its probes are collected.
    pub(super) sink: Sink<'a>,
}

impl<'a> Checker<'a> {
    fn text(&self, tok: &Token) -> &'a str {
        &self.src[tok.span.start..tok.span.end]
    }

    /// The offset of the `val` keyword that declared `name`, or `None`
    /// when the innermost binding of that name is an ordinary one (or
    /// there is none). Innermost wins — that is the shadowing rule.
    fn lookup(&self, frames: &[Frame<'a>], name: &str) -> Option<usize> {
        for frame in frames.iter().rev() {
            if let Some(var) = frame.vars.iter().rev().find(|v| v.name == name) {
                return var.val_at;
            }
        }
        None
    }

    /// Walks one token stream (the file, or one template interpolation),
    /// pushing and popping scopes as it goes. Frames pushed here are
    /// dropped on the way out, so an interpolation cannot leak scopes into
    /// the stream that contains it.
    pub(super) fn walk(&self, tokens: &'a [Token], frames: &mut Vec<Frame<'a>>) {
        let base = frames.len();
        // Parameter scopes, activated when the walk reaches the function
        // body they belong to: (body start, body end, bindings).
        let mut pending: Vec<(usize, usize, Vec<Var<'a>>)> = Vec::new();
        let mut i = 0usize;
        while i < tokens.len() {
            while frames.len() > base && frames[frames.len() - 1].end <= i {
                frames.pop();
            }
            while let Some(pos) = pending.iter().position(|(start, _, _)| *start <= i) {
                let (_, end, vars) = pending.remove(pos);
                frames.push(Frame { end, vars });
            }

            let tok = &tokens[i];
            match &tok.kind {
                TokenKind::Template(parts) => {
                    for part in parts.iter() {
                        if let TplPart::Interp { tokens: inner, .. } = part {
                            self.walk(inner, frames);
                        }
                    }
                }
                TokenKind::Punct(b'{') => {
                    let end = find_close_at(tokens, i).unwrap_or(tokens.len());
                    frames.push(Frame {
                        end,
                        vars: Vec::new(),
                    });
                }
                TokenKind::Punct(b'(') => {
                    if let Some((start, end, vars)) = self.param_scope(tokens, i) {
                        // A `val` parameter is a `val` binding like any
                        // other; it just never goes through `declare`.
                        if let Sink::Probes(sink) = self.sink {
                            sink.borrow_mut().bindings.extend(
                                vars.iter()
                                    .filter(|v| v.val_at.is_some())
                                    .map(|v| ValBinding {
                                        name: v.name.to_string(),
                                        ident: v.ident,
                                        val_at: v.val_at.expect("filtered val binding"),
                                    }),
                            );
                        }
                        pending.push((start, end, vars));
                    }
                }
                // prefix `++x.foo` / `--x.foo`
                TokenKind::Punct(b'+' | b'-') if incdec_at(tokens, i) => {
                    if matches!(tokens.get(i + 2).map(|t| &t.kind), Some(TokenKind::Ident))
                        && !dotted_at(tokens, 0, i + 2)
                    {
                        self.check_mutation(tokens, i + 2, frames, true);
                    }
                }
                TokenKind::Ident => {
                    i = self.visit_ident(tokens, i, frames);
                    continue;
                }
                _ => {}
            }
            i += 1;
        }
        frames.truncate(base);
    }

    /// Handles one identifier token: declarations register bindings, uses
    /// are checked for mutation and for call-site capability. Returns the
    /// index to continue from.
    fn visit_ident(&self, tokens: &'a [Token], i: usize, frames: &mut Vec<Frame<'a>>) -> usize {
        let word = self.text(&tokens[i]);
        if dotted_at(tokens, 0, i) {
            return i + 1;
        }

        if word == "val"
            && let Some(kind) = modifier_at(self.src, tokens, i)
        {
            return match kind {
                // the parameter's scope is registered at its `(`
                ValModifier::Parameter => i + 1,
                ValModifier::Declaration => {
                    let names = collect_decl_names(self.src, tokens, i + 2);
                    self.declare(frames, names, Some(tokens[i].span.start));
                    i + 2
                }
            };
        }

        match word {
            "const" | "let" | "var" => {
                let names = collect_decl_names(self.src, tokens, i + 1);
                self.declare(frames, names, None);
                return i + 1;
            }
            "function" | "class" => {
                if let Some(t) = tokens.get(i + 1)
                    && matches!(t.kind, TokenKind::Ident)
                {
                    self.declare(frames, vec![self.text(t)], None);
                }
                return i + 1;
            }
            // a `for` head's bindings belong to the loop, not to the
            // enclosing block
            "for" => {
                if punct_at(tokens, i + 1, b'(')
                    && let Some(close) = find_close_at(tokens, i + 1)
                {
                    frames.push(Frame {
                        end: statement_end(tokens, close + 1),
                        vars: Vec::new(),
                    });
                }
                return i + 1;
            }
            "delete" => {
                if matches!(tokens.get(i + 1).map(|t| &t.kind), Some(TokenKind::Ident))
                    && !dotted_at(tokens, 0, i + 1)
                {
                    self.check_mutation(tokens, i + 1, frames, true);
                }
                return i + 1;
            }
            _ => {}
        }

        self.check_mutation(tokens, i, frames, false);
        if punct_at(tokens, i + 1, b'(') {
            match self.sink {
                // Which declaration a call names is the checker's question:
                // every call to a name the file declares is collected, and
                // the pairing is by symbol identity, so a name the untyped
                // path has to call ambiguous is settled per call site. The
                // name gate only skips calls no same-file declaration
                // could possibly match.
                Sink::Probes(_) if self.signatures.contains_key(word) => {
                    self.probe_call(tokens, i, word);
                }
                _ => {
                    if let Some(Some(params)) = self.signatures.get(word) {
                        self.check_call(tokens, i + 1, word, params, frames);
                    }
                }
            }
        }
        i + 1
    }

    /// Registers bindings in the innermost scope.
    fn declare(&self, frames: &mut [Frame<'a>], names: Vec<&'a str>, val_at: Option<usize>) {
        let src = self.src;
        if let Some(frame) = frames.last_mut() {
            frame.vars.extend(names.into_iter().map(|name| Var {
                name,
                val_at,
                ident: offset_in(src, name),
            }));
        }
        if val_at.is_some()
            && let Sink::Probes(sink) = self.sink
            && let Some(frame) = frames.last()
        {
            // Every `val` binding is a node the checker can resolve; which
            // mutations belong to it is then a question of symbol identity,
            // not of this file's scope model.
            sink.borrow_mut().bindings.extend(
                frame
                    .vars
                    .iter()
                    .filter(|v| v.val_at == val_at)
                    .map(|v| ValBinding {
                        name: v.name.to_string(),
                        ident: v.ident,
                        val_at: v.val_at.expect("filtered val binding"),
                    }),
            );
        }
    }

    /// The scope a parameter list opens, when the `(` at `open` really is
    /// one: `(bindings' scope start, scope end, bindings)`. A parameter
    /// list is told from a call or a grouping by what follows its `)` — a
    /// body brace, an arrow, or a return type — and control-flow heads
    /// (`if (`, `while (`, ...) are excluded by the word in front of it.
    /// `catch (e)` *is* a binding form and is deliberately included.
    fn param_scope(
        &self,
        tokens: &'a [Token],
        open: usize,
    ) -> Option<(usize, usize, Vec<Var<'a>>)> {
        if open > 0
            && let TokenKind::Ident = tokens[open - 1].kind
        {
            // A control-flow head (`if (c) { ... }`) or a tt `match` is
            // not a parameter list even though a block follows it.
            // `function`/`async` do introduce one, and `catch (e)` really
            // is a binding form, so those three stay in.
            let word = self.text(&tokens[open - 1]);
            if word == "match"
                || (is_reserved(word) && !matches!(word, "function" | "async" | "catch"))
            {
                return None;
            }
        }
        let close = find_close_at(tokens, open)?;
        let body = self.body_after_params(tokens, close + 1)?;
        let vars = parse_params(self.src, tokens, open)
            .into_iter()
            .zip(list_entries(tokens, open))
            .flat_map(|(param, (start, end))| {
                let val_at = param.is_val.then(|| tokens[start].span.start);
                let mut names = Vec::new();
                let mut k = start;
                while k < end
                    && matches!(&tokens[k].kind, TokenKind::Ident
                        if self.text(&tokens[k]) == "val" || is_param_modifier(self.text(&tokens[k])))
                {
                    k += 1;
                }
                collect_pattern_names(self.src, tokens, k, &mut names);
                let src = self.src;
                names.into_iter().map(move |name| Var {
                    name,
                    val_at,
                    ident: offset_in(src, name),
                })
            })
            .collect();
        Some((body.0, body.1, vars))
    }

    /// The `(start, end)` token range of a function body that follows a
    /// parameter list's `)`, or `None` when what follows is not one (so
    /// the parens were a call or a grouping).
    fn body_after_params(&self, tokens: &[Token], mut k: usize) -> Option<(usize, usize)> {
        if punct_at(tokens, k, b':') {
            // a return type annotation — skip to the body brace or arrow,
            // stepping over object types (`: { a: number } {`)
            k += 1;
            loop {
                match tokens.get(k)?.kind {
                    TokenKind::Arrow => break,
                    TokenKind::Punct(b'{') => {
                        let type_brace = k > 0
                            && matches!(
                                tokens[k - 1].kind,
                                TokenKind::Punct(b':' | b'|' | b'&' | b'<' | b'(' | b'[' | b',')
                            );
                        if !type_brace {
                            break;
                        }
                        k = find_close_at(tokens, k)? + 1;
                    }
                    TokenKind::Punct(b'(' | b'[') => k = find_close_at(tokens, k)? + 1,
                    TokenKind::Punct(b')' | b']' | b'}' | b';' | b',') => return None,
                    _ => k += 1,
                }
            }
        }
        if matches!(tokens.get(k)?.kind, TokenKind::Arrow) {
            k += 1;
        }
        if punct_at(tokens, k, b'{') {
            return Some((k, find_close_at(tokens, k)?));
        }
        // an arrow with an expression body: the scope runs to the end of
        // that expression
        if matches!(
            tokens.get(k.wrapping_sub(1)).map(|t| &t.kind),
            Some(TokenKind::Arrow)
        ) {
            return Some((k, expression_end(tokens, k)));
        }
        None
    }

    /// Reports a mutation through the access path rooted at the identifier
    /// token `root`, when that identifier resolves to a `val` binding.
    /// `mutates` is set by callers that already know the path is being
    /// mutated by an operator *in front* of it (`delete x.p`, `++x.p`);
    /// otherwise the operator after the path decides.
    fn check_mutation(&self, tokens: &[Token], root: usize, frames: &[Frame<'a>], mutates: bool) {
        let name = self.text(&tokens[root]);
        // In probe mode the root is *not* resolved here: which binding it
        // names is the checker's answer, from the symbol at this identifier.
        if !matches!(self.sink, Sink::Probes(_)) && self.lookup(frames, name).is_none() {
            return;
        }
        let path = parse_path(self.src, tokens, root);
        if path.steps == 0 {
            // replacing the binding's value is `const`'s business
            return;
        }
        let offset = tokens[root].span.start;
        if mutates || assignment_op_at(tokens, path.end).is_some() || incdec_at(tokens, path.end) {
            match self.sink {
                Sink::Probes(sink) => sink.borrow_mut().mutations.push(Mutation {
                    root: offset,
                    name: name.to_string(),
                    method: None,
                }),
                Sink::Report(sink) => sink.borrow_mut().push(
                    TtError::span(
                        offset,
                        offset + name.len(),
                        format!(
                            "cannot mutate through val binding `{name}` \
                             (the binding is declared with `val`, so every access path from it is read-only)"
                        ),
                    )
                    .code(crate::DiagnosticCode::ValMutation),
                ),
            }
            return;
        }
        // A method call is a *question*: `q.set(k)` mutates only if `q` is
        // a built-in with a mutating `set`, which needs the receiver's
        // type. ttc never answers it from the name — it records the call
        // for `ttc --types`, where the real checker decides.
        if let (Some(method), Some(tok)) = (path.last_prop, path.last_prop_tok)
            && punct_at(tokens, path.end, b'(')
        {
            match self.sink {
                // Every call is collected; which ones count is the verdict's
                // business ([`is_builtin_mutator_name`]), so a name outside
                // the policy can never hide a question from the checker.
                Sink::Probes(sink) => sink.borrow_mut().mutations.push(Mutation {
                    root: offset,
                    name: name.to_string(),
                    method: Some((method.to_string(), tokens[tok].span.start)),
                }),
                Sink::Report(_) => {}
            }
        }
    }

    /// Checks the arguments of a call to a function declared in this file:
    /// a `val` binding may only be passed to a parameter that is itself
    /// declared `val`. Only plain access-path arguments carry a decidable
    /// capability; anything computed is left alone.
    fn check_call(
        &self,
        tokens: &[Token],
        open: usize,
        callee: &str,
        params: &[ParamSig],
        frames: &[Frame<'a>],
    ) {
        let Sink::Report(report) = self.sink else {
            return; // probes go through `probe_call`; Calls asks nothing
        };
        for (idx, (start, end)) in list_entries(tokens, open).into_iter().enumerate() {
            if !matches!(tokens[start].kind, TokenKind::Ident) || dotted_at(tokens, 0, start) {
                continue;
            }
            let name = self.text(&tokens[start]);
            if self.lookup(frames, name).is_none() {
                continue;
            }
            // only `x` / `x.y.z` — a computed argument is not a path
            let path = parse_path(self.src, tokens, start);
            if path.end != end || (path.steps > 0 && path.last_prop.is_none()) {
                continue;
            }
            let Some(param) = params.get(idx) else {
                continue;
            };
            if param.is_val {
                continue;
            }
            let described = match &param.name {
                Some(n) => format!("`{n}`"),
                None => format!("#{}", idx + 1),
            };
            report.borrow_mut().push(
                TtError::span(
                    tokens[start].span.start,
                    tokens[start].span.end,
                    format!(
                        "cannot pass val binding `{name}` to mutable parameter {described} of \
                         `{callee}` (the parameter is not declared with `val`, so the function \
                         may mutate through it)"
                    ),
                )
                .code(crate::DiagnosticCode::ValPass),
            );
        }
    }

    /// Collects the plain-path arguments of a call to a name the file
    /// declares, with the callee identifier's position — the capability
    /// question a checker settles by pairing that callee's symbol with a
    /// declaration's ([`ValPass`]). Nothing is decided here: which
    /// declaration is called, whether the argument is a `val` binding, and
    /// which parameter it lands on are all the verdict's half.
    fn probe_call(&self, tokens: &[Token], callee: usize, word: &str) {
        let Sink::Probes(sink) = self.sink else {
            return;
        };
        for (idx, (start, end)) in list_entries(tokens, callee + 1).into_iter().enumerate() {
            if !matches!(tokens[start].kind, TokenKind::Ident) || dotted_at(tokens, 0, start) {
                continue;
            }
            // only `x` / `x.y.z` — a computed argument is not a path
            let path = parse_path(self.src, tokens, start);
            if path.end != end || (path.steps > 0 && path.last_prop.is_none()) {
                continue;
            }
            sink.borrow_mut().passes.push(ValPass {
                offset: tokens[start].span.start,
                name: self.text(&tokens[start]).to_string(),
                callee: word.to_string(),
                callee_at: tokens[callee].span.start,
                arg_index: idx,
            });
        }
    }
}
