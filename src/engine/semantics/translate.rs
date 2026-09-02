//! TypeScript diagnostic translation and tt type naming.

use super::*;

/// Turns a TypeScript diagnostic that landed on ttc's own glue into a tt
/// one — said in tt's words, about tt's construct.
///
/// The pair `(construct, error code)` is the whole key. It is deliberately
/// a whitelist: an unrecognized diagnostic passes through unchanged.
pub(crate) fn translate(
    kind: AnchorKind,
    code: u32,
    message: &str,
    declarations: &[DeclaredVariant],
) -> Option<String> {
    translation_class(kind, code)?;
    let said = match (kind, code) {
        // `.kind` / `.value` reached for on something that is not a Result.
        (AnchorKind::Try, 2339 | 2551 | 2571) => {
            "`try` needs a `Result` — this expression is not one".to_string()
        }
        (AnchorKind::Result, 2339 | 2551 | 2571) => {
            "the `try` in this `result` block needs a `Result` — this expression is not one"
                .to_string()
        }
        // The propagated `Err` reaching a return type that cannot hold it.
        (AnchorKind::Try, 2322 | 2345) => "the `Err` this `try` propagates does not fit the \
             enclosing function's return type — tt has no automatic conversion, so widen the \
             return type or convert the error"
            .to_string(),
        (AnchorKind::LetElse, 2339 | 2571) => {
            "let-else needs a value with a `kind` discriminant — this expression has none"
                .to_string()
        }
        (AnchorKind::IfLet, 2339 | 2571) => {
            "`if let` needs a value with a `kind` discriminant — this expression has none"
                .to_string()
        }
        (AnchorKind::Match, 2339 | 2571) => {
            "match on a tag pattern needs a value with a `kind` discriminant — this scrutinee \
             has none (a plain TypeScript `enum` is not one)"
                .to_string()
        }
        // A tag compared against a type that cannot hold it. ttc reports
        // the ones that look like misspellings itself; this is the rest.
        (AnchorKind::Match, 2678) | (AnchorKind::LetElse | AnchorKind::IfLet, 2367) => {
            "this pattern's case is not one the value can be".to_string()
        }
        // The value flowing into a pipeline step does not fit the step. The
        // span already names the rejecting step (the per-step anchor); the
        // sentence names the two sides.
        (AnchorKind::Pipe, 2345) => match assignability_pair(message) {
            Some((found, expected)) => {
                format!("this pipeline step expects `{expected}`, but receives `{found}`")
            }
            None => "this pipeline step cannot accept the value flowing into it".to_string(),
        },
        _ => return None,
    };
    let message = message.trim();
    match name_types(message, declarations) {
        Some(named) => Some(format!(
            "{said} (in tt's names: {named}) (ts{code}: {message})"
        )),
        None => Some(format!("{said} (ts{code}: {message})")),
    }
}

/// Stable meaning shared by CLI and editor translation deduplication.
/// TypeScript may emit several incidental diagnostics for one tt mistake;
/// the class identifies the single tt-level explanation they share.
pub(crate) fn translation_class(kind: AnchorKind, code: u32) -> Option<&'static str> {
    match (kind, code) {
        (AnchorKind::Try | AnchorKind::Result, 2339 | 2551 | 2571) => Some("not-result"),
        (AnchorKind::Try, 2322 | 2345) => Some("try-error-type"),
        (AnchorKind::LetElse | AnchorKind::IfLet | AnchorKind::Match, 2339 | 2571) => {
            Some("missing-discriminant")
        }
        (AnchorKind::Match, 2678) | (AnchorKind::LetElse | AnchorKind::IfLet, 2367) => {
            Some("impossible-case")
        }
        (AnchorKind::Pipe, 2345) => Some("pipe-step-input"),
        _ => None,
    }
}

/// The tightest `'found' is not assignable to … type 'expected'` pair in a
/// checker message. TypeScript's elaboration lines descend from the outer
/// types toward the incompatible leaf, so the last pair is the most
/// specific one — the same reduction [`TypeMismatch::differences`] proves
/// structurally when the checker's structured facts are available.
pub(super) fn assignability_pair(message: &str) -> Option<(String, String)> {
    const NEEDLE: &str = "' is not assignable to ";
    let mut result = None;
    let mut from = 0;
    while let Some(at) = message[from..].find(NEEDLE) {
        let at = from + at;
        from = at + NEEDLE.len();
        let found = message[..at].rfind('\'').map(|open| &message[open + 1..at]);
        let tail = &message[from..];
        let tail = tail.strip_prefix("parameter of ").unwrap_or(tail);
        let expected = tail
            .strip_prefix("type '")
            .and_then(|rest| rest.find('\'').map(|close| &rest[..close]));
        if let (Some(found), Some(expected)) = (found, expected) {
            result = Some((found.to_string(), expected.to_string()));
        }
    }
    result
}

/// A checker message in tt's vocabulary.
///
/// The TypeScript code is *not* spelled into the sentence: it is already
/// [`Diagnostic::code`], and a consumer that wants to show it reads it
/// there. Repeating it here would make the rendered form say it twice
/// (`error[ts2322]: ts(2322): …`), and the only way back out would be to
/// recognise the prefix by its shape (TASK-213 decision 6).
pub(super) fn ts_message(message: &str, declarations: &[DeclaredVariant]) -> String {
    match name_types(message, declarations) {
        Some(named) => format!("{message} (in tt's names: {named})"),
        None => message.to_string(),
    }
}

pub(super) fn named_type(text: &str, declarations: &[DeclaredVariant]) -> String {
    name_types(text, declarations).unwrap_or_else(|| text.to_string())
}

/// Renders checker-owned assignability facts without depending on the text
/// or nesting of a TypeScript diagnostic message. This is the common CLI and
/// editor wording; the raw checker message is only the fallback when the
/// backend could not prove an expected/found relation.
pub(super) fn diagnostic_message(
    diagnostic: &TsDiagnostic,
    declarations: &[DeclaredVariant],
) -> String {
    // Property lookup already names the exact member and receiver type. A
    // contextual mismatch synthesized around the enclosing generated
    // destructuring is wider and reverses the user's expected/found reading;
    // keep the checker's direct, source-mappable fact instead.
    if matches!(diagnostic.code, 2339 | 2551) {
        return ts_message(&diagnostic.message, declarations);
    }
    let Some(mismatch) = &diagnostic.mismatch else {
        return ts_message(&diagnostic.message, declarations);
    };
    let (expected, found, required) = mismatch_pair(mismatch, declarations);
    let mut message = format!("type mismatch: expected `{expected}`, found `{found}`");
    if let Some(required) = required {
        message.push_str(&format!("\n  required type: `{required}`"));
    }
    message
}

/// Whether an anchor is a pipeline *step* anchor — the per-step input
/// position, which alone justifies the step-boundary wording. The anchor
/// covering a whole pipeline is the same kind but carries no producer
/// context: a mismatch there is about the pipeline's result in its
/// surrounding position (an argument, an annotation), not about any step.
pub(super) fn pipe_step_anchor(anchor: &crate::EmitAnchor) -> bool {
    anchor.kind == AnchorKind::Pipe && anchor.context.is_some()
}

/// [`diagnostic_message`], said in the vocabulary of the construct whose
/// glue the diagnostic landed on. A pipeline's per-step anchor already
/// underlines the step that rejected the value, so its mismatch reads as
/// what that step expects versus what the pipeline feeds it; every other
/// anchor — the whole-pipeline one included — keeps the generic wording.
pub(super) fn anchored_diagnostic_message(
    anchor: &crate::EmitAnchor,
    diagnostic: &TsDiagnostic,
    declarations: &[DeclaredVariant],
) -> String {
    if pipe_step_anchor(anchor)
        && let Some(mismatch) = &diagnostic.mismatch
    {
        let (mut expected, mut found, mut required) = mismatch_pair(mismatch, declarations);
        // A `flow` boundary mismatches as two function types the structural
        // reduction does not descend into; the checker's own elaboration
        // does, and its deepest pair is the boundary's value types.
        let unreduced = expected == named_type(&mismatch.expected, declarations)
            && found == named_type(&mismatch.found, declarations);
        if unreduced
            && let Some((found_leaf, expected_leaf)) = assignability_pair(&diagnostic.message)
        {
            let expected_leaf = named_type(&expected_leaf, declarations);
            let found_leaf = named_type(&found_leaf, declarations);
            if expected_leaf != expected || found_leaf != found {
                required = Some(expected);
                expected = expected_leaf;
                found = found_leaf;
            }
        }
        let mut message =
            format!("this pipeline step expects `{expected}`, but receives `{found}`");
        if let Some(required) = required {
            message.push_str(&format!("\n  required type: `{required}`"));
        }
        return message;
    }
    diagnostic_message(diagnostic, declarations)
}

/// The expected/found pair a structured mismatch renders: the minimal
/// incompatible leaves when the checker reduced to a single expected type,
/// else the complete pair — plus the complete contextual type when the pair
/// shown was reduced from it.
pub(super) fn mismatch_pair(
    mismatch: &TypeMismatch,
    declarations: &[DeclaredVariant],
) -> (String, String, Option<String>) {
    let expected = named_type(&mismatch.expected, declarations);
    let found = named_type(&mismatch.found, declarations);
    let differences: Vec<(String, String)> = mismatch
        .differences
        .iter()
        .map(|difference| {
            (
                named_type(&difference.expected, declarations),
                named_type(&difference.found, declarations),
            )
        })
        .collect();
    let leaf_expected = differences.first().map(|pair| pair.0.as_str());
    let one_expected = leaf_expected.is_some()
        && differences
            .iter()
            .all(|pair| Some(pair.0.as_str()) == leaf_expected);
    // `one_expected` is `leaf_expected.is_some() && …`, so the binding is
    // the same test read as a value.
    if let Some(expected_leaf) = leaf_expected.filter(|_| one_expected) {
        let mut found_leaves: Vec<&str> = Vec::new();
        for (_, leaf) in &differences {
            if !found_leaves.contains(&leaf.as_str()) {
                found_leaves.push(leaf);
            }
        }
        let found_leaf = found_leaves.join(" | ");
        let required = (expected_leaf != expected || found_leaf != found).then(|| expected.clone());
        return (expected_leaf.to_string(), found_leaf, required);
    }
    (expected, found, None)
}

/// The secondary places a checker diagnostic points at, in `.tt`
/// coordinates: the construct anchor's companion span first (a pipeline's
/// producing step), then the checker's own related information, each mapped
/// back through the same origin machinery the primary span traveled. A
/// related place in a file the snapshot does not hold (a lib file, an
/// uncompiled dependency) is dropped rather than guessed at.
pub(super) fn checker_labels(
    files: &[Arc<ProjectedDocument>],
    host: &ProjectedDocument,
    anchor: Option<&crate::EmitAnchor>,
    diagnostic: &TsDiagnostic,
) -> Vec<DiagnosticLabel> {
    let mut labels = Vec::new();
    if let Some(anchor) = anchor
        && anchor.kind == AnchorKind::Pipe
        && let Some((start, end)) = anchor.context
    {
        labels.push(DiagnosticLabel {
            path: None,
            position: crate::line_col(&host.source, start),
            end: crate::line_col(&host.source, end),
            message: "the piped value is produced here".to_string(),
        });
    }
    for related in &diagnostic.related {
        let Some(file) = files.iter().find(|f| f.module_path == related.file) else {
            continue;
        };
        let Some(origin) = projection::diagnostic_origin(file, related.start, related.end) else {
            continue;
        };
        let (start, end) = match origin {
            DiagnosticOrigin::Exact { start, end } => (start, end.max(start.saturating_add(1))),
            DiagnosticOrigin::Anchor(anchor) => (anchor.src, anchor.src_end),
            DiagnosticOrigin::Nearest { start } => (start, start.saturating_add(1)),
        };
        labels.push(DiagnosticLabel {
            path: (file.source_path != host.source_path).then(|| file.source_path.clone()),
            position: crate::line_col(&file.source, start),
            end: crate::line_col(&file.source, end),
            message: related.message.clone(),
        });
    }
    labels
}

pub(super) fn diagnostic_span(diagnostic: &TsDiagnostic) -> (usize, usize) {
    if matches!(diagnostic.code, 2339 | 2551) {
        return (diagnostic.start, diagnostic.end);
    }
    diagnostic
        .mismatch
        .as_ref()
        .map_or((diagnostic.start, diagnostic.end), |mismatch| {
            (mismatch.start, mismatch.end)
        })
}

pub(super) fn finish_diagnostics(mut diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    diagnostics.sort_by(|left, right| {
        (&left.path, left.position, left.end, &left.message).cmp(&(
            &right.path,
            right.position,
            right.end,
            &right.message,
        ))
    });
    diagnostics.dedup_by(|right, left| {
        left.path == right.path
            && left.position == right.position
            && left.end == right.end
            && left.message == right.message
    });
    diagnostics
}

/// The message again, with every structural case type the declaration table
/// **uniquely** recognizes written as the tt name it lowers from — `None`
/// when it recognizes none of them and the restatement would be the message
/// itself.
///
/// TypeScript has no word for a tt case: `Test.OutOfRange` lowers to a
/// member of a union type, so a diagnostic about one prints the member —
/// `{ kind: "OutOfRange"; value: number; }` — and the reader is left to
/// match it back against a declaration by eye. tt can do that matching: the
/// tag and the payload's field names name a case, and the table says which
/// variant declaration declares it.
///
/// Two rules keep the restatement honest, and both are the whitelist rule
/// of [`translate`] again:
///
/// - A tag two variants in scope declare is **not** named. The point is to say
///   which declaration this is, and a guess between two says nothing.
/// - A member whose field names are not exactly the case's is not named
///   either — it is some other type that happens to carry the same tag.
///
/// A union of members that covers every case of one variant is the variant
/// itself (`ParseError`), which is how the return type in the motivating
/// example comes back to its declared name; a partial union stays a union
/// of named cases (`ParseError.NotANumber | ParseError.Overflow`).
///
/// The restatement is a reading aid and never replaces the original — the
/// caller carries TypeScript's own text along with it, because a name tt
/// got wrong has to be checkable against what the checker actually said.
pub(crate) fn name_types(message: &str, declarations: &[DeclaredVariant]) -> Option<String> {
    let members = case_members(message, declarations);
    if members.is_empty() {
        return None;
    }
    let mut out = String::new();
    let mut at = 0;
    for run in union_runs(message, &members) {
        let (first, last) = (&members[run.start], &members[run.end - 1]);
        out.push_str(&message[at..first.start]);
        match collapsed(&members[run.start..run.end], declarations) {
            Some(name) => out.push_str(&name),
            None => {
                let named: Vec<String> = members[run.start..run.end]
                    .iter()
                    .map(|m| format!("{}.{}", m.variant_name, m.tag))
                    .collect();
                out.push_str(&named.join(" | "));
            }
        }
        at = last.end;
    }
    out.push_str(&message[at..]);
    Some(out)
}

/// One structural case type found in a message, resolved to its
/// declaration.
pub(super) struct CaseMember {
    /// Byte span of the `{ ... }` in the message.
    start: usize,
    end: usize,
    /// The variant that declares the case, under the name the analyzed file
    /// calls it by.
    variant_name: String,
    /// The case's tag.
    tag: String,
}

/// A `[start, end)` range of members that a message writes as one union.
pub(super) struct Run {
    start: usize,
    end: usize,
}

/// Every structural case type in `message`, in order, that the table names.
///
/// The scan is quote-aware — a `{` inside a string literal opens nothing —
/// and it descends into an object type it does not recognize, so a case
/// nested in some other type is still found.
pub(super) fn case_members(message: &str, declarations: &[DeclaredVariant]) -> Vec<CaseMember> {
    let bytes = message.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'"' => at = string_end(bytes, at),
            b'{' => match object_end(bytes, at) {
                Some(end) => match recognize(&message[at..end], declarations) {
                    Some((variant_name, tag)) => {
                        out.push(CaseMember {
                            start: at,
                            end,
                            variant_name,
                            tag,
                        });
                        at = end;
                    }
                    // Not a case of its own — but one may be written
                    // inside it.
                    None => at += 1,
                },
                None => at += 1,
            },
            _ => at += 1,
        }
    }
    out
}

/// The members grouped into the unions the message writes them as: members
/// separated by exactly `" | "` belong to one union.
pub(super) fn union_runs(message: &str, members: &[CaseMember]) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for (i, member) in members.iter().enumerate() {
        match runs.last_mut() {
            Some(run) if &message[members[i - 1].end..member.start] == " | " => run.end = i + 1,
            _ => runs.push(Run {
                start: i,
                end: i + 1,
            }),
        }
    }
    runs
}

/// The variant name a whole union collapses to — `Some` only when its members
/// are the cases of one variant, each once, all of them.
pub(super) fn collapsed(run: &[CaseMember], declarations: &[DeclaredVariant]) -> Option<String> {
    let first = run.first()?;
    if run.iter().any(|m| m.variant_name != first.variant_name) {
        return None;
    }
    let declared = declarations.iter().find(|d| d.name == first.variant_name)?;
    if declared.constructors.len() != run.len() {
        return None;
    }
    declared
        .constructors
        .iter()
        .all(|c| run.iter().any(|m| m.tag == c.tag))
        .then(|| first.variant_name.clone())
}

/// The case an object type names — `Some((variant, tag))` when exactly one
/// declaration in scope declares the tag *and* its payload is exactly the
/// fields written here.
pub(super) fn recognize(text: &str, declarations: &[DeclaredVariant]) -> Option<(String, String)> {
    let (tag, mut fields) = object_fields(text)?;
    fields.sort_unstable();
    let mut candidates = declarations
        .iter()
        .filter(|d| d.constructors.iter().any(|c| c.tag == tag));
    let declared = candidates.next()?;
    if candidates.next().is_some() {
        return None; // two declarations answer to the tag — neither is *the* one
    }
    let constructor = declared.constructors.iter().find(|c| c.tag == tag)?;
    let mut declared_fields: Vec<&str> = constructor
        .fields
        .iter()
        .flatten()
        .map(|f| f.name.as_str())
        .collect();
    declared_fields.sort_unstable();
    (declared_fields == fields).then(|| (declared.name.clone(), tag))
}

/// An object type's `kind` tag and the names of its other members — `None`
/// when it has no string-literal `kind`, or is written in a shape this
/// reader does not fully understand (a method, an index signature).
pub(super) fn object_fields(text: &str) -> Option<(String, Vec<&str>)> {
    let inner = text.strip_prefix('{')?.strip_suffix('}')?;
    let mut tag = None;
    let mut fields = Vec::new();
    for entry in split_top(inner, b';') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let colon = split_top(entry, b':').next()?.len();
        if colon >= entry.len() {
            return None; // no `name: type` — not a property
        }
        let name = entry[..colon].trim().trim_end_matches('?');
        let value = entry[colon + 1..].trim();
        if !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$')
        {
            return None; // an index signature, a quoted name, a call
        }
        if name == "kind" {
            let literal = value.strip_prefix('"')?.strip_suffix('"')?;
            if literal.contains(['"', '|']) {
                return None; // a union of tags, not one case
            }
            tag = Some(literal.to_string());
        } else {
            fields.push(name);
        }
    }
    Some((tag?, fields))
}

/// `text` split on `sep` where nothing encloses it — a `;` inside a nested
/// object, a bracket or a string separates nothing.
pub(super) fn split_top(text: &str, sep: u8) -> impl Iterator<Item = &str> {
    let bytes = text.as_bytes();
    let mut cuts = Vec::new();
    let (mut at, mut depth, mut start) = (0, 0usize, 0);
    while at < bytes.len() {
        match bytes[at] {
            b'"' => {
                at = string_end(bytes, at);
                continue;
            }
            b'{' | b'(' | b'[' => depth += 1,
            b'}' | b')' | b']' => depth = depth.saturating_sub(1),
            b if b == sep && depth == 0 => {
                cuts.push(&text[start..at]);
                start = at + 1;
            }
            _ => {}
        }
        at += 1;
    }
    cuts.push(&text[start..]);
    cuts.into_iter()
}

/// The byte just past the `}` that closes the object type opening at
/// `at` — `None` when nothing closes it.
pub(super) fn object_end(bytes: &[u8], at: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut i = at;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                i = string_end(bytes, i);
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The byte just past the string literal opening at `at`.
pub(super) fn string_end(bytes: &[u8], at: usize) -> usize {
    let quote = bytes[at];
    let mut i = at + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b if b == quote => return i + 1,
            _ => i += 1,
        }
    }
    bytes.len()
}

/// Source membership for a typed report. TypeScript's configured program is
/// authoritative for project roots; explicit inputs are roots by user
/// request. Relative tt imports then extend that set through the language's
/// own module graph, including a source that could not be projected and
/// therefore could never appear in TypeScript's source-file table.
pub(super) fn typed_member_sources(
    snapshot: &Snapshot,
    answers: &Answers,
    requested: &HashSet<PathBuf>,
) -> Option<HashSet<PathBuf>> {
    let modules = answers.project_modules.as_ref()?;
    let configured: HashSet<&std::path::Path> = modules.iter().map(PathBuf::as_path).collect();
    let mut members: HashSet<PathBuf> = snapshot
        .files()
        .iter()
        .filter(|file| configured.contains(file.module_path.as_path()))
        .map(|file| file.source_path.clone())
        .chain(
            snapshot
                .blocked()
                .iter()
                .filter(|file| {
                    configured
                        .contains(super::projection::module_path_of(&file.source_path).as_path())
                })
                .map(|file| file.source_path.clone()),
        )
        .chain(requested.iter().cloned())
        .collect();
    let mut pending: Vec<PathBuf> = members.iter().cloned().collect();

    while let Some(source) = pending.pop() {
        let imports = snapshot
            .files()
            .iter()
            .find(|file| file.source_path == source)
            .map(|file| file.tt_imports())
            .or_else(|| {
                snapshot
                    .blocked()
                    .iter()
                    .find(|file| file.source_path == source)
                    .map(|file| file.tt_imports())
            });
        let Some(imports) = imports else {
            continue;
        };
        let directory = source.parent().unwrap_or(std::path::Path::new("."));
        for import in imports {
            let Ok(target) = directory.join(&import.specifier).canonicalize() else {
                continue;
            };
            let held = snapshot
                .files()
                .iter()
                .any(|file| file.source_path == target)
                || snapshot
                    .blocked()
                    .iter()
                    .any(|file| file.source_path == target);
            if held && members.insert(target.clone()) {
                pending.push(target);
            }
        }
    }

    Some(members)
}
