//! Semantic results in tt's own vocabulary.
//!
//! The checker's answers come back as [`crate::typescript::backend::Answers`]
//! — coordinates in emitted modules, symbol ids, raw diagnostics. This module
//! turns them into what a consumer of the engine actually wants: diagnostics
//! at positions in the `.tt` source, with the exact wording the CLI has
//! always printed, and declarations matched back to the files they were
//! emitted for. Nothing TypeScript-shaped leaves the engine.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use super::projection::{self, Probes, ProjectedDocument};
use super::snapshot::Snapshot;
use crate::AnchorKind;
use crate::analysis::{DeclaredVariant, PayloadAlphabet};
use crate::typescript::backend::{Answers, Diagnostic as TsDiagnostic, Resolution, TypeMismatch};
use crate::typescript::mapper::DiagnosticOrigin;

/// One reported problem, at a position in a file the user can open.
///
/// The message carries its full wording, so a consumer prints or displays
/// it verbatim and two consumers can never drift apart on phrasing. It says
/// what is wrong and nothing else: the rule's identity is `code` and the
/// fix is `suggestions`, so neither has to be read back out of the
/// sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The file the problem is in — an `.tt` source, or a hand-written
    /// TypeScript file when the checker reported one of those.
    pub path: PathBuf,
    /// 1-based line and column (columns count UTF-8 code points), or `None`
    /// when the diagnostic has no position in a file the user wrote.
    pub position: Option<(usize, usize)>,
    /// Where the diagnostic's range ends — 1-based line and column, past
    /// the last character it covers. `None` when only a position is known;
    /// a consumer that draws a range then decides its own width (the
    /// editor underlines the word at the position).
    ///
    /// This is what makes a diagnostic point at the *construct*: `try
    /// parse(text)`, `match (shape)`, the mutated binding — the same span
    /// Rust underlines, rather than the first character of the statement.
    pub end: Option<(usize, usize)>,
    /// The full message, as it is shown.
    pub message: String,
    /// The diagnostic's stable identity: a tt rule's code
    /// ([`crate::DiagnosticCode::as_str`], e.g. `match-not-exhaustive`) or
    /// a TypeScript code (`ts2322`). `None` only where no rule is known.
    /// The same rule carries the same code on every path — CLI, server,
    /// editor, typed or untyped.
    pub code: Option<String>,
    /// How to resolve the problem, in the same form the untyped pipeline
    /// reports it ([`crate::Diagnostic::suggestions`]).
    ///
    /// An [`crate::Edit`]'s offsets are byte offsets into the source of the
    /// file named by `path`, so a consumer holding that text can apply one
    /// without asking the compiler again. Carrying the field here is what
    /// keeps the typed path from silently dropping a fix the untyped pass
    /// already computed.
    pub suggestions: Vec<crate::Suggestion>,
    /// Secondary places this diagnostic points at, each with its own words
    /// — rustc's labeled spans ("the piped value comes from this step",
    /// "the expected type comes from this declaration"). Empty for a
    /// diagnostic that has only its primary span.
    pub labels: Vec<DiagnosticLabel>,
}

/// One secondary span of a [`Diagnostic`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    /// The file the span is in, when it is not the diagnostic's own file.
    pub path: Option<PathBuf>,
    /// 1-based line and column of the label's start.
    pub position: (usize, usize),
    /// 1-based line and column just past the label's range.
    pub end: (usize, usize),
    /// What this place explains.
    pub message: String,
}

/// The typed pass's copy of the advice that closes a match — the same
/// constant the untyped pass attaches, so the two pipelines cannot drift
/// apart on the fix any more than they can on the wording.
/// Where the match a probe asked about is written — the shape the shared
/// arm-insertion authoring takes.
fn site_of(anchor: &projection::MatchAnchor) -> crate::diagnostics::MatchSite {
    crate::diagnostics::MatchSite {
        keyword_off: anchor.anchor.offset,
        body_open: anchor.body_open,
        body_close: anchor.body_close,
    }
}

/// The advice with no edit behind it — what a match whose body braces did
/// not reach this pass still gets to say.
fn non_exhaustive_help() -> crate::Suggestion {
    crate::Suggestion {
        message: format!(
            "{} {}",
            crate::diagnostics::NON_EXHAUSTIVE_HELP,
            crate::diagnostics::NON_EXHAUSTIVE_WILDCARD_HELP
        ),
        edit: None,
    }
}

/// What one checked snapshot came back with.
#[derive(Debug, Default)]
pub struct Checked {
    /// Every diagnostic of the pass, in report order: the tt layer first
    /// (each file's own recoverable diagnostics), then the type layer
    /// (unless the request was tt-only), literal exhaustiveness, tag
    /// exhaustiveness, `val` mutations, and `val` passes.
    pub diagnostics: Vec<Diagnostic>,
    /// The declarations the compiler emitted, when they were requested.
    pub declarations: Declarations,
    /// Why the TypeScript layer could not answer, when it could not — the
    /// backend failed to run (no toolchain, a dead process). The tt-level
    /// diagnostics above are still complete: a missing type checker
    /// removes the *typed* facts, not tt's own answers
    /// (`docs/design/compiler-core.md` §7).
    pub backend_error: Option<String>,
}

/// The declarations of one emitting pass, matched back to their sources.
#[derive(Debug, Default)]
pub struct Declarations {
    /// The standard library package declarations, keyed by physical module.
    pub std: Vec<StdDeclaration>,
    /// One entry per requested `.tt` file the compiler emitted for.
    pub modules: Vec<ModuleDeclaration>,
}

/// One emitted declaration module of `@tt/std`.
#[derive(Debug)]
pub struct StdDeclaration {
    /// Which standard-library module this declaration describes.
    pub module: crate::StdModule,
    /// The declaration text emitted by TypeScript.
    pub text: String,
}

/// One lowered module's declarations, paired with the file they belong to.
#[derive(Debug)]
pub struct ModuleDeclaration {
    /// The projected file the declarations were emitted for.
    pub file: Arc<ProjectedDocument>,
    /// The declaration text — the compiler's own; only the sidecar map is
    /// ttc's to build.
    pub text: String,
}

/// One match's alphabets as the checker named them: where the `match`
/// keyword is, and each scrutinee position's constituents in position
/// order (a single match has one).
type MatchAlphabets = (usize, Vec<Vec<String>>);

/// One file's semantic analysis, computed once and cached across
/// snapshots by [`super::Project`] — the report consumes this instead of
/// re-parsing the file (and everything it imports) on every pass.
///
/// The cache key is the pair (file content, imported declarations): a
/// change to a dependency's *body* leaves this valid, a change to its
/// exported declarations invalidates it — exactly the invalidation
/// boundary the query plan names (`docs/design/compiler-core.md` §11).
#[derive(Debug)]
pub(crate) struct FileSemantics {
    /// The imported declarations in this file's scope (aliases applied) —
    /// half of the cache key, and `checked_coverage`'s input.
    pub externs: Vec<crate::VariantSymbol>,
    /// The file's pattern analyses over those externs.
    pub analyses: crate::PatternAnalyses,
}

/// Turns a TypeScript diagnostic that landed on ttc's own glue into a tt
/// one — said in tt's words, about tt's construct.
///
/// This is the third layer of `docs/design/rust-parity-analysis.md` §10 and
/// the point of [`crate::EmitAnchor`]. The error-layer contract asks that a
/// user never meet a TypeScript error caused by code ttc wrote; until now
/// that state was not reached — the diagnostic simply arrived wearing the
/// generated code's face. Translating it reaches it.
///
/// The pair `(construct, error code)` is the whole key. It is deliberately
/// a **whitelist**: a diagnostic this table does not recognize is passed
/// through unchanged rather than guessed at, because silently restating an
/// error as something it is not is worse than an ugly message. The original
/// text rides along in every translation for the same reason.
///
/// `declarations` is the file's declaration table ([`crate::pattern_analyses`]).
/// It is what lets the translation say `Test.OutOfRange` where TypeScript
/// wrote `{ kind: "OutOfRange"; value: number; }` — see [`name_types`].
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
        (AnchorKind::ResultBind, 2339 | 2551 | 2571) => {
            "`<-` needs a `Result` — this expression is not one".to_string()
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
        (AnchorKind::Try | AnchorKind::ResultBind, 2339 | 2551 | 2571) => Some("not-result"),
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
fn assignability_pair(message: &str) -> Option<(String, String)> {
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
fn ts_message(message: &str, declarations: &[DeclaredVariant]) -> String {
    match name_types(message, declarations) {
        Some(named) => format!("{message} (in tt's names: {named})"),
        None => message.to_string(),
    }
}

fn named_type(text: &str, declarations: &[DeclaredVariant]) -> String {
    name_types(text, declarations).unwrap_or_else(|| text.to_string())
}

/// Renders checker-owned assignability facts without depending on the text
/// or nesting of a TypeScript diagnostic message. This is the common CLI and
/// editor wording; the raw checker message is only the fallback when the
/// backend could not prove an expected/found relation.
fn diagnostic_message(diagnostic: &TsDiagnostic, declarations: &[DeclaredVariant]) -> String {
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
fn pipe_step_anchor(anchor: &crate::EmitAnchor) -> bool {
    anchor.kind == AnchorKind::Pipe && anchor.context.is_some()
}

/// [`diagnostic_message`], said in the vocabulary of the construct whose
/// glue the diagnostic landed on. A pipeline's per-step anchor already
/// underlines the step that rejected the value, so its mismatch reads as
/// what that step expects versus what the pipeline feeds it; every other
/// anchor — the whole-pipeline one included — keeps the generic wording.
fn anchored_diagnostic_message(
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
fn mismatch_pair(
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
fn checker_labels(
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

fn diagnostic_span(diagnostic: &TsDiagnostic) -> (usize, usize) {
    diagnostic
        .mismatch
        .as_ref()
        .map_or((diagnostic.start, diagnostic.end), |mismatch| {
            (mismatch.start, mismatch.end)
        })
}

fn finish_diagnostics(mut diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
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
struct CaseMember {
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
struct Run {
    start: usize,
    end: usize,
}

/// Every structural case type in `message`, in order, that the table names.
///
/// The scan is quote-aware — a `{` inside a string literal opens nothing —
/// and it descends into an object type it does not recognize, so a case
/// nested in some other type is still found.
fn case_members(message: &str, declarations: &[DeclaredVariant]) -> Vec<CaseMember> {
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
fn union_runs(message: &str, members: &[CaseMember]) -> Vec<Run> {
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
fn collapsed(run: &[CaseMember], declarations: &[DeclaredVariant]) -> Option<String> {
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
fn recognize(text: &str, declarations: &[DeclaredVariant]) -> Option<(String, String)> {
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
fn object_fields(text: &str) -> Option<(String, Vec<&str>)> {
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
fn split_top(text: &str, sep: u8) -> impl Iterator<Item = &str> {
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
fn object_end(bytes: &[u8], at: usize) -> Option<usize> {
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
fn string_end(bytes: &[u8], at: usize) -> usize {
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

/// Builds the pass's diagnostics from the checker's answers, in the exact
/// order and wording the report has always had.
pub(crate) fn report(
    snapshot: &Snapshot,
    answers: &Answers,
    probes: &Probes,
    tt_only: bool,
    semantics: &HashMap<PathBuf, Arc<FileSemantics>>,
) -> Vec<Diagnostic> {
    let files = snapshot.files();
    let mut out = Vec::new();

    for file in snapshot.blocked() {
        for diagnostic in &file.diagnostics {
            out.push(Diagnostic {
                path: file.source_path.clone(),
                position: diagnostic.start.map(|at| crate::line_col(&file.source, at)),
                end: diagnostic.end.map(|at| crate::line_col(&file.source, at)),
                message: diagnostic.message.clone(),
                code: Some(diagnostic.code.as_str().to_string()),
                suggestions: diagnostic.suggestions.clone(),
                labels: Vec::new(),
            });
        }
    }

    // The tt layer first: the diagnostics each file's projection found on
    // its own (duplicate arms, unknown cases, misplaced constructs). They
    // are tt's answers about tt's constructs, so they are reported on the
    // tt-only path too — and they no longer gate the rest of this report
    // (TASK-117 symptom 3): the typed answers below follow either way.
    for file in files {
        for d in &file.tt_diagnostics {
            out.push(Diagnostic {
                path: file.source_path.clone(),
                position: d.start.map(|at| crate::line_col(&file.source, at)),
                end: d.end.map(|at| crate::line_col(&file.source, at)),
                message: d.message.clone(),
                code: Some(d.code.as_str().to_string()),
                suggestions: d.suggestions.clone(),
                labels: Vec::new(),
            });
        }
    }

    // A projection is deliberately file-local, so it cannot resolve names
    // against declarations imported from another source. The cached semantic
    // file can. Render those answers through sema's one diagnostic author and
    // merge them with the projection results; local names may appear in both,
    // while imported names appear only here.
    for file in files {
        let Some(semantics) = semantics.get(&file.source_path) else {
            continue;
        };
        for error in crate::sema::resolution_errors(&semantics.analyses) {
            let diagnostic = Diagnostic {
                path: file.source_path.clone(),
                position: error.offset.map(|at| crate::line_col(&file.source, at)),
                end: error.end.map(|at| crate::line_col(&file.source, at)),
                message: error.message,
                code: Some(error.code.as_str().to_string()),
                suggestions: error.suggestions,
                labels: Vec::new(),
            };
            if !out.contains(&diagnostic) {
                out.push(diagnostic);
            }
        }
    }

    // TypeScript's own diagnostics, at the position in the `.tt` file the
    // offending code was written at.
    let type_diagnostics: &[TsDiagnostic] = if tt_only { &[] } else { &answers.diagnostics };
    let structured_glue: HashSet<(PathBuf, usize, AnchorKind)> = type_diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.mismatch.is_some())
        .filter_map(|diagnostic| {
            let file = files
                .iter()
                .find(|file| file.module_path == diagnostic.file)?;
            let (start, end) = diagnostic_span(diagnostic);
            let DiagnosticOrigin::Anchor(anchor) = projection::diagnostic_origin(file, start, end)?
            else {
                return None;
            };
            Some((file.source_path.clone(), anchor.src, anchor.kind))
        })
        .collect();
    let mut translated_seen: HashSet<(PathBuf, usize, AnchorKind, &'static str)> = HashSet::new();
    for diagnostic in type_diagnostics {
        let (diagnostic_start, diagnostic_end) = diagnostic_span(diagnostic);
        let Some(file) = files.iter().find(|f| f.module_path == diagnostic.file) else {
            // A hand-written file: TypeScript's own coordinates already name
            // a file the user can open, so they are used as they are.
            out.push(Diagnostic {
                path: diagnostic.file.clone(),
                position: None,
                end: None,
                message: diagnostic_message(diagnostic, &[]),
                code: Some(format!("ts{}", diagnostic.code)),
                suggestions: Vec::new(),
                labels: Vec::new(),
            });
            continue;
        };
        if projection::diagnostic_intersects_recovery(file, diagnostic) {
            continue;
        }
        if projection::diagnostic_intersects_tt_error(file, diagnostic) {
            continue;
        }
        let Some(origin) = projection::diagnostic_origin(file, diagnostic_start, diagnostic_end)
        else {
            out.push(Diagnostic {
                path: file.source_path.clone(),
                position: None,
                end: None,
                message: diagnostic_message(diagnostic, &[]),
                code: Some(format!("ts{}", diagnostic.code)),
                suggestions: Vec::new(),
                labels: Vec::new(),
            });
            continue;
        };
        // Glue is not the user's code. When ttc can say what the construct
        // meant, it says that — over the construct's own text. The
        // declaration table its wording names types from is built only for
        // a file that has a diagnostic on glue at all, and once for it.
        if let DiagnosticOrigin::Anchor(anchor) = origin {
            if anchor.kind == AnchorKind::Match
                && semantics.get(&file.source_path).is_some_and(|semantics| {
                    semantics.analyses.match_has_resolution_error(anchor.src)
                })
            {
                continue;
            }
            let structured_key = (file.source_path.clone(), anchor.src, anchor.kind);
            if structured_glue.contains(&structured_key) {
                // Several checker diagnostics can describe one failed
                // lowering. The contextual expected/found relation is the
                // cause; property and comparison errors on the same glue are
                // consequences and must not become separate user errors.
                if diagnostic.mismatch.is_none()
                    || !translated_seen.insert((
                        file.source_path.clone(),
                        anchor.src,
                        anchor.kind,
                        "structured-type-mismatch",
                    ))
                {
                    continue;
                }
                let declared: &[DeclaredVariant] = semantics
                    .get(&file.source_path)
                    .map(|s| s.analyses.declarations.as_slice())
                    .unwrap_or_default();
                out.push(Diagnostic {
                    path: file.source_path.clone(),
                    position: Some(crate::line_col(&file.source, anchor.src)),
                    end: Some(crate::line_col(&file.source, anchor.src_end)),
                    message: anchored_diagnostic_message(&anchor, diagnostic, declared),
                    code: Some(format!("ts{}", diagnostic.code)),
                    suggestions: Vec::new(),
                    labels: checker_labels(files, file, Some(&anchor), diagnostic),
                });
                continue;
            }
            // The whole-pipeline anchor shares its kind with the step
            // anchors but not their meaning: only a step anchor may speak
            // in step vocabulary.
            let translates = anchor.kind != AnchorKind::Pipe || pipe_step_anchor(&anchor);
            if translates
                && let Some(class) = translation_class(anchor.kind, diagnostic.code)
                && !translated_seen.insert((
                    file.source_path.clone(),
                    anchor.src,
                    anchor.kind,
                    class,
                ))
            {
                continue;
            }
            let declared: &[DeclaredVariant] = semantics
                .get(&file.source_path)
                .map(|s| s.analyses.declarations.as_slice())
                .unwrap_or_default();
            if translates
                && let Some(said) =
                    translate(anchor.kind, diagnostic.code, &diagnostic.message, declared)
            {
                let entry = Diagnostic {
                    path: file.source_path.clone(),
                    position: Some(crate::line_col(&file.source, anchor.src)),
                    end: Some(crate::line_col(&file.source, anchor.src_end)),
                    message: said,
                    code: Some(format!("ts{}", diagnostic.code)),
                    suggestions: Vec::new(),
                    labels: checker_labels(files, file, Some(&anchor), diagnostic),
                };
                // One construct's glue can draw several TypeScript errors
                // that all mean the same tt thing (`$tt_t.kind` and
                // `$tt_t.value`).
                if !out.contains(&entry) {
                    out.push(entry);
                }
                continue;
            }
        }
        let declared: &[DeclaredVariant] = semantics
            .get(&file.source_path)
            .map(|s| s.analyses.declarations.as_slice())
            .unwrap_or_default();
        match origin {
            DiagnosticOrigin::Exact { start, end } => {
                out.push(Diagnostic {
                    path: file.source_path.clone(),
                    position: Some(crate::line_col(&file.source, start)),
                    end: (end > start).then(|| crate::line_col(&file.source, end)),
                    message: diagnostic_message(diagnostic, declared),
                    code: Some(format!("ts{}", diagnostic.code)),
                    suggestions: Vec::new(),
                    labels: checker_labels(files, file, None, diagnostic),
                });
            }
            DiagnosticOrigin::Anchor(anchor) => out.push(Diagnostic {
                path: file.source_path.clone(),
                position: Some(crate::line_col(&file.source, anchor.src)),
                end: Some(crate::line_col(&file.source, anchor.src_end)),
                message: format!(
                    "{} (in code ttc generated for this construct)",
                    diagnostic_message(diagnostic, declared)
                ),
                code: Some(format!("ts{}", diagnostic.code)),
                suggestions: Vec::new(),
                labels: checker_labels(files, file, Some(&anchor), diagnostic),
            }),
            DiagnosticOrigin::Nearest { start } => out.push(Diagnostic {
                path: file.source_path.clone(),
                position: Some(crate::line_col(&file.source, start)),
                end: None,
                message: format!(
                    "{} (in code ttc generated near this position)",
                    diagnostic_message(diagnostic, declared)
                ),
                code: Some(format!("ts{}", diagnostic.code)),
                suggestions: Vec::new(),
                labels: checker_labels(files, file, None, diagnostic),
            }),
        }
    }

    // Literal-match exhaustiveness, decided by the type TypeScript computes
    // at the scrutinee — narrowing included.
    for missing in &answers.literal_missing {
        let Some(anchor) = probes.literals.get(missing.index) else {
            continue;
        };
        let Some(file) = files
            .iter()
            .find(|f| f.source_path == anchor.anchor.source_path)
        else {
            continue;
        };
        // A literal arm is written as the value itself, so the witness the
        // checker names *is* the arm pattern — the same text the message
        // quotes, which is why both come from `display_literal`.
        let uncovered: Vec<String> = missing.missing.iter().map(display_literal).collect();
        out.push(Diagnostic {
            path: file.source_path.clone(),
            position: Some(crate::line_col(&file.source, anchor.anchor.offset)),
            end: Some(crate::line_col(&file.source, anchor.anchor.end)),
            message: crate::diagnostics::non_exhaustive_message(
                Some("literal union"),
                &uncovered,
                false,
            ),
            code: Some(
                crate::DiagnosticCode::MatchNotExhaustive
                    .as_str()
                    .to_string(),
            ),
            suggestions: crate::diagnostics::non_exhaustive_suggestions(
                &file.source,
                site_of(anchor),
                &uncovered,
            ),
            labels: Vec::new(),
        });
    }

    // Tag exhaustiveness. The checker names the constituents the
    // scrutinee's type still has — narrowing included — and tt runs its
    // own algorithm over that alphabet, which is what sees a hole *inside*
    // a payload as well as a missing case (TASK-108).
    //
    // A witness tt is not certain of is dropped here: the default path
    // reports those because it has nothing better, but on this path the
    // honest answer for an unidentifiable column is to ask the checker,
    // and that question is not asked yet.
    // Per file, per match: the alphabet of each scrutinee position, in
    // position order (a single match has one).
    let mut by_file: HashMap<PathBuf, Vec<MatchAlphabets>> = HashMap::new();
    // The payload answers ride in the same list, after the match ones —
    // they name the alphabet of a `(constructor, field)` column, which is
    // the one thing tt cannot work out from declarations alone.
    let mut payloads: HashMap<PathBuf, Vec<PayloadAlphabet>> = HashMap::new();
    // `(file, match keyword) -> end of `match (scrutinee)``.
    let mut match_ends: HashMap<(PathBuf, usize), usize> = HashMap::new();
    // The same key -> where the match's body braces are, so a coverage
    // hole's fix can be written as an edit on this path too.
    let mut sites: HashMap<(PathBuf, usize), crate::diagnostics::MatchSite> = HashMap::new();
    for members in &answers.tag_members {
        if let Some(anchor) = probes.tags.get(members.index) {
            let per_match = by_file
                .entry(anchor.anchor.source_path.clone())
                .or_default();
            match per_match
                .iter_mut()
                .find(|(at, _)| *at == anchor.anchor.offset)
            {
                Some((_, positions)) => positions.push(members.tags.clone()),
                None => per_match.push((anchor.anchor.offset, vec![members.tags.clone()])),
            }
            // The keyword offset keys the alphabets; the range it opens is
            // what the diagnostic underlines, and the braces are where its
            // fix is written.
            sites.insert(
                (anchor.anchor.source_path.clone(), anchor.anchor.offset),
                site_of(anchor),
            );
            match_ends.insert(
                (anchor.anchor.source_path.clone(), anchor.anchor.offset),
                anchor.anchor.end,
            );
            continue;
        }
        let Some(anchor) = probes
            .payloads
            .get(members.index.wrapping_sub(probes.tags.len()))
        else {
            continue;
        };
        payloads
            .entry(anchor.source_path.clone())
            .or_default()
            .push((
                (anchor.tag.clone(), anchor.field.clone()),
                members.tags.clone(),
            ));
    }
    for file in files {
        let Some(asked) = by_file.get(&file.source_path) else {
            continue;
        };
        // The nested columns are resolved from declarations, so the
        // imported ones have to be collected — otherwise a payload whose
        // type is an imported variant reads as an unknown alphabet and its
        // holes go unreported. The cached semantics carry them.
        let externs: &[crate::VariantSymbol] = semantics
            .get(&file.source_path)
            .map(|s| s.externs.as_slice())
            .unwrap_or_default();
        let asked_payloads = payloads
            .get(&file.source_path)
            .map_or(&[][..], Vec::as_slice);
        for (offset, coverage) in
            crate::analysis::checked_coverage(&file.source, externs, asked, asked_payloads)
        {
            if semantics
                .get(&file.source_path)
                .is_some_and(|semantics| semantics.analyses.match_has_resolution_error(offset))
            {
                continue;
            }
            // A single match's witness is one pattern, quoted the way the
            // default path quotes one; a tuple match's is a combination of
            // positions, written as one `(a, b)` and left unquoted — the
            // quotes would read as part of the pattern.
            let uncovered: Vec<String> = coverage
                .missing
                .iter()
                .filter(|m| m.certain)
                .map(|m| {
                    if m.pattern.len() > 1 {
                        format!("({})", m.pattern.join(", "))
                    } else {
                        format!("{:?}", m.pattern.first().cloned().unwrap_or_default())
                    }
                })
                .collect();
            if uncovered.is_empty() {
                continue;
            }
            // The arms that close the hole, from the same witnesses in
            // their binding form — one authoring, both pipelines.
            let arms: Vec<String> = coverage
                .missing
                .iter()
                .filter(|m| m.certain)
                .map(|m| {
                    if m.arm.len() > 1 {
                        format!("({})", m.arm.join(", "))
                    } else {
                        m.arm.first().cloned().unwrap_or_else(|| "_".to_string())
                    }
                })
                .collect();
            // The typed pass knows the alphabet but not the declaration,
            // so the shared renderer gets no subject — one renderer, one
            // wording, on both pipelines (TASK-120).
            let tuple = coverage.positions.len() > 1;
            out.push(Diagnostic {
                path: file.source_path.clone(),
                position: Some(crate::line_col(&file.source, offset)),
                end: match_ends
                    .get(&(file.source_path.clone(), offset))
                    .map(|at| crate::line_col(&file.source, *at)),
                message: crate::diagnostics::non_exhaustive_message(None, &uncovered, tuple),
                code: Some(
                    crate::DiagnosticCode::MatchNotExhaustive
                        .as_str()
                        .to_string(),
                ),
                suggestions: match sites.get(&(file.source_path.clone(), offset)) {
                    Some(site) => {
                        crate::diagnostics::non_exhaustive_suggestions(&file.source, *site, &arms)
                    }
                    None => vec![non_exhaustive_help()],
                },
                labels: Vec::new(),
            });
        }
    }

    // `val`: two resolutions decide, and ttc guesses neither of them.
    //
    // 1. Which binding a path is rooted at — the root identifier and the
    //    binding's declaration are the same binding when they are the same
    //    symbol. Shadowing, redeclaration and destructuring come out right
    //    because this is TypeScript's own resolution, not a model of it.
    // 2. For a method call, whether the method is a built-in — declared in
    //    TypeScript's own lib files. A user-defined method that shares the
    //    name is not, and anything unresolved is left alone.
    let symbols: HashMap<usize, &Resolution> =
        answers.resolutions.iter().map(|r| (r.index, r)).collect();
    let val_symbols: HashSet<i64> = probes
        .val_bindings
        .iter()
        .filter_map(|i| symbols.get(i).map(|r| r.id))
        .collect();

    for mutation in &probes.mutations {
        let Some(root) = symbols.get(&mutation.root) else {
            continue; // unresolved — never a verdict
        };
        if !val_symbols.contains(&root.id) {
            continue; // not this binding, whatever it is called
        }
        if let Some(method) = mutation.method {
            match symbols.get(&method) {
                // Two halves make the verdict: the checker's — the
                // method is one of TypeScript's own — and tt's policy —
                // that method is one of the mutating ones. A built-in
                // `get` fails the second; a user-defined `set`, or a
                // method the checker could not resolve, fails the first.
                Some(resolution)
                    if resolution.builtin && crate::is_builtin_mutator_name(&resolution.name) => {}
                _ => continue,
            }
        }
        let Some(file) = files
            .iter()
            .find(|f| f.source_path == mutation.anchor.source_path)
        else {
            continue;
        };
        let message = match &mutation.method_name {
            // The built-in itself is not named: the compiler answered
            // "this method is one of TypeScript's own", which is the
            // verdict — not which interface declares it.
            Some(method) => format!(
                "cannot call mutating method `{}` through val binding `{}` \
                 (the binding is declared with `val`, so every access path from it is \
                 read-only)",
                method, mutation.name,
            ),
            None => format!(
                "cannot mutate through val binding `{}` \
                 (the binding is declared with `val`, so every access path \
                 from it is read-only)",
                mutation.name,
            ),
        };
        out.push(Diagnostic {
            path: file.source_path.clone(),
            position: Some(crate::line_col(&file.source, mutation.anchor.offset)),
            end: Some(crate::line_col(&file.source, mutation.anchor.end)),
            message,
            code: Some(crate::DiagnosticCode::ValMutation.as_str().to_string()),
            suggestions: Vec::new(),
            labels: Vec::new(),
        });
    }

    // The callee table: a declaration's symbol names its parameter
    // list. One symbol carrying declarations with *different* lists
    // (TypeScript overloads, `var` merging) makes that callee
    // ambiguous, and an ambiguous callee is not judged — the same
    // caution the name-keyed table of the untyped path takes, here at
    // symbol granularity, so two functions merely sharing a name stay
    // two callees.
    let mut callees: HashMap<i64, Option<&[crate::ValParam]>> = HashMap::new();
    for function in &probes.functions {
        let Some(resolution) = symbols.get(&function.root) else {
            continue;
        };
        match callees.get(&resolution.id) {
            Some(Some(prev)) if *prev == function.params.as_slice() => {}
            Some(_) => {
                callees.insert(resolution.id, None);
            }
            None => {
                callees.insert(resolution.id, Some(&function.params));
            }
        }
    }

    // The function boundary: a `val` binding may only be handed to a
    // parameter that is itself `val`. Which binding the argument names,
    // and which declaration the call names, are the same symbol
    // question the mutations above ask — an unresolved callee, or one
    // no collected declaration matches (an import, a method), is never
    // a verdict.
    for pass in &probes.passes {
        let Some(root) = symbols.get(&pass.root) else {
            continue;
        };
        if !val_symbols.contains(&root.id) {
            continue;
        }
        let Some(callee) = symbols.get(&pass.callee_symbol) else {
            continue;
        };
        let Some(Some(params)) = callees.get(&callee.id) else {
            continue;
        };
        let Some(param) = params.get(pass.arg_index) else {
            continue;
        };
        if param.is_val {
            continue;
        }
        let described = match &param.name {
            Some(name) => format!("`{name}`"),
            None => format!("#{}", pass.arg_index + 1),
        };
        let Some(file) = files
            .iter()
            .find(|f| f.source_path == pass.anchor.source_path)
        else {
            continue;
        };
        out.push(Diagnostic {
            path: file.source_path.clone(),
            position: Some(crate::line_col(&file.source, pass.anchor.offset)),
            end: Some(crate::line_col(&file.source, pass.anchor.end)),
            message: format!(
                "cannot pass val binding `{}` to mutable parameter {} of \
                 `{}` (the parameter is not declared with `val`, so the function may mutate \
                 through it)",
                pass.name, described, pass.callee,
            ),
            code: Some(crate::DiagnosticCode::ValPass.as_str().to_string()),
            suggestions: Vec::new(),
            labels: Vec::new(),
        });
    }

    finish_diagnostics(out)
}

/// Matches the compiler's emitted declarations back to the snapshot's files.
/// Only `requested` files are kept — the rest of the project is in the graph
/// so that it resolves, not so that it is emitted.
pub(crate) fn match_declarations(
    snapshot: &Snapshot,
    answers: &Answers,
    root: &std::path::Path,
    requested: &HashSet<PathBuf>,
) -> Declarations {
    let mut out = Declarations::default();
    // The standard library's own declarations, so a consumer running plain
    // tsc can map every `@tt/std` entry to them. They are project modules
    // like any other, but have no `.tt` sources to sit beside.
    for declaration in &answers.declarations {
        if let Some(module) = crate::StdModule::ALL.into_iter().find(|module| {
            declaration.path
                == root
                    .join(projection::std_module_path(*module))
                    .with_extension("d.ts")
        }) {
            out.std.push(StdDeclaration {
                module,
                text: declaration.text.clone(),
            });
            continue;
        }
        let Some(file) = snapshot
            .files()
            .iter()
            .find(|f| projection::declaration_path_of(f) == declaration.path)
            .filter(|f| requested.contains(&f.source_path))
        else {
            continue;
        };
        out.modules.push(ModuleDeclaration {
            file: file.clone(),
            text: declaration.text.clone(),
        });
    }
    out
}

/// The variant declarations one file's direct `.tt` imports bring into scope,
/// preferring the snapshot's own text for a file it holds — the same
/// 1-hop collection every other surface does, so an imported variant is known
/// under the name the import gave it.
/// The imported declarations in `file`'s scope, read from the snapshot's
/// cached per-file symbols where the import target is in the snapshot —
/// a target that did not change is never re-parsed — and from disk
/// otherwise.
pub(crate) fn externs_of(
    snapshot: &Snapshot,
    file: &ProjectedDocument,
) -> Vec<crate::VariantSymbol> {
    super::language::externs_from(&file.source_path, file.tt_imports(), &|target| {
        snapshot
            .files()
            .iter()
            .find(|f| f.source_path == target)
            .map(|f| {
                f.variant_symbols()
                    .iter()
                    .filter(|d| d.exported)
                    .cloned()
                    .collect()
            })
            .or_else(|| {
                snapshot
                    .blocked()
                    .iter()
                    .find(|f| f.source_path == target)
                    .map(|f| {
                        f.variant_symbols()
                            .iter()
                            .filter(|d| d.exported)
                            .cloned()
                            .collect()
                    })
            })
            .or_else(|| {
                let text = std::fs::read_to_string(target).ok()?;
                Some(
                    crate::variant_symbols_with_kind(
                        &text,
                        crate::SourceKind::from_path(target).unwrap_or_default(),
                    )
                    .into_iter()
                    .filter(|d| d.exported)
                    .collect(),
                )
            })
    })
}

/// A covered literal as it reads in a message.
fn display_literal(literal: &crate::Literal) -> String {
    match literal {
        crate::Literal::String(s) => format!("{s:?}"),
        crate::Literal::Number(n) => n.to_string(),
        crate::Literal::BigInt(d) => format!("{d}n"),
        crate::Literal::Boolean(b) => b.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The declaration table of a source, as a translation sees it.
    fn table(source: &str) -> Vec<DeclaredVariant> {
        crate::pattern_analyses(source, &[]).declarations
    }

    /// The motivating example of TASK-118: the propagated `Err` and the
    /// return type it does not fit, both said as the user declared them.
    #[test]
    fn a_case_is_named_and_a_full_union_is_its_variant() {
        let declarations = table(
            "variant Test { OutOfRange(value: number), Empty }\n\
             variant ParseError { NotANumber(text: string) }\n",
        );
        let said = translate(
            AnchorKind::Try,
            2322,
            "Type 'Err<{ kind: \"OutOfRange\"; value: number; }>' is not assignable to type \
             'Result<string, { kind: \"NotANumber\"; text: string; }>'.",
            &declarations,
        )
        .expect("translated");
        assert!(
            said.contains(
                "(in tt's names: Type 'Err<Test.OutOfRange>' is not assignable to type \
                 'Result<string, ParseError>'.)"
            ),
            "{said}"
        );
        // The original rides along, unchanged — the names are checkable.
        assert!(
            said.contains("(ts2322: Type 'Err<{ kind: \"OutOfRange\""),
            "{said}"
        );
    }

    #[test]
    fn a_partial_union_stays_a_union_of_named_cases() {
        let declarations = table("variant E { A(x: number), B, C }\n");
        let named = name_types(
            "Type '{ kind: \"A\"; x: number; } | { kind: \"B\"; }' is not assignable to type 'E'.",
            &declarations,
        )
        .expect("named");
        assert_eq!(
            named,
            "Type 'E.A | E.B' is not assignable to type 'E'.".to_string()
        );
    }

    #[test]
    fn a_tag_two_declarations_answer_to_is_not_named() {
        let declarations = table("variant A { Empty }\nvariant B { Empty }\n");
        assert_eq!(
            name_types("Type '{ kind: \"Empty\"; }'.", &declarations),
            None
        );
    }

    #[test]
    fn a_type_that_only_shares_a_tag_is_not_named() {
        // Same tag, different payload — some other type, not this case.
        let declarations = table("variant E { A(x: number) }\n");
        assert_eq!(
            name_types("Type '{ kind: \"A\"; y: string; }'.", &declarations),
            None
        );
    }

    #[test]
    fn a_message_with_no_structural_case_is_left_alone() {
        let declarations = table("variant E { A(x: number), B }\n");
        assert_eq!(
            name_types(
                "Property 'kind' does not exist on type 'number'.",
                &declarations
            ),
            None,
        );
        // ... and a translation of it carries no naming clause.
        let said = translate(
            AnchorKind::Try,
            2339,
            "Property 'kind' does not exist.",
            &declarations,
        )
        .expect("translated");
        assert!(!said.contains("in tt's names"), "{said}");
    }

    #[test]
    fn a_case_nested_in_another_type_is_named_where_it_stands() {
        let declarations = table("variant E { A(x: number), B }\n");
        let named = name_types(
            "Type 'Array<{ kind: \"A\"; x: number; }>' is not assignable.",
            &declarations,
        )
        .expect("named");
        assert_eq!(named, "Type 'Array<E.A>' is not assignable.".to_string());
    }

    #[test]
    fn a_tag_union_names_nothing() {
        // `{ kind: "A" | "B" }` is not one case, so it is not one name.
        let declarations = table("variant E { A(x: number), B }\n");
        assert_eq!(
            name_types("Type '{ kind: \"A\" | \"B\"; }'.", &declarations),
            None
        );
    }

    #[test]
    fn translation_classes_group_incidental_ts_codes_by_tt_meaning() {
        assert_eq!(translation_class(AnchorKind::Try, 2339), Some("not-result"));
        assert_eq!(translation_class(AnchorKind::Try, 2571), Some("not-result"));
        assert_eq!(
            translation_class(AnchorKind::Try, 2322),
            Some("try-error-type")
        );
        assert_eq!(translation_class(AnchorKind::Pipe, 2339), None);
        assert_eq!(
            translation_class(AnchorKind::Pipe, 2345),
            Some("pipe-step-input")
        );
    }

    #[test]
    fn a_pipe_mismatch_translates_to_the_steps_expectation() {
        let said = translate(
            AnchorKind::Pipe,
            2345,
            "Argument of type 'number' is not assignable to parameter of type 'string'.",
            &[],
        )
        .expect("translated");
        assert!(
            said.starts_with("this pipeline step expects `string`, but receives `number`"),
            "{said}"
        );
        assert!(said.contains("ts2345:"), "the original rides along: {said}");
    }

    #[test]
    fn a_pipe_translation_uses_the_deepest_elaborated_pair() {
        // A flow boundary mismatches as two function types; the elaboration
        // descends to the value types, and the deepest pair is the boundary.
        let said = translate(
            AnchorKind::Pipe,
            2345,
            "Argument of type '(n: number) => number' is not assignable to parameter of type \
             '(n: number) => string'.\n  Type 'number' is not assignable to type 'string'.",
            &[],
        )
        .expect("translated");
        assert!(
            said.starts_with("this pipeline step expects `string`, but receives `number`"),
            "{said}"
        );
    }

    #[test]
    fn an_unparseable_pipe_mismatch_still_says_what_the_step_meant() {
        let said =
            translate(AnchorKind::Pipe, 2345, "something unusual.", &[]).expect("translated");
        assert!(
            said.starts_with("this pipeline step cannot accept the value flowing into it"),
            "{said}"
        );
    }

    #[test]
    fn assignability_pairs_parse_both_sentence_forms() {
        assert_eq!(
            assignability_pair("Argument of type 'A' is not assignable to parameter of type 'B'."),
            Some(("A".to_string(), "B".to_string()))
        );
        assert_eq!(
            assignability_pair("Type 'A' is not assignable to type 'B'."),
            Some(("A".to_string(), "B".to_string()))
        );
        assert_eq!(assignability_pair("This expression is not callable."), None);
    }

    #[test]
    fn an_ordinary_ts_message_also_uses_tt_names() {
        let declarations = table("variant E { A(x: number), B }\n");
        let said = ts_message(
            "Type '{ kind: \"A\"; x: number; }' is not assignable.",
            &declarations,
        );
        assert_eq!(
            said,
            "Type '{ kind: \"A\"; x: number; }' is not assignable. \
             (in tt's names: Type 'E.A' is not assignable.)"
        );
    }

    #[test]
    fn diagnostics_are_source_sorted_and_display_duplicates_are_merged() {
        let at = |line, code: &str| Diagnostic {
            path: PathBuf::from("/p/a.tt"),
            position: Some((line, 1)),
            end: Some((line, 2)),
            message: "same".to_string(),
            code: Some(code.to_string()),
            suggestions: Vec::new(),
            labels: Vec::new(),
        };
        let finished = finish_diagnostics(vec![at(2, "ts9999"), at(1, "ts1000"), at(2, "ts1001")]);
        assert_eq!(finished.len(), 2);
        assert_eq!(finished[0].position, Some((1, 1)));
        assert_eq!(finished[1].position, Some((2, 1)));
    }
}
