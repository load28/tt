//! Semantic results in tt's own vocabulary.
//!
//! The checker's answers come back as [`crate::typescript::backend::Answers`]
//! — coordinates in emitted modules, symbol ids, raw diagnostics. This module
//! turns them into what a consumer of the engine actually wants: diagnostics
//! at positions in the `.tt` source, with the exact wording the CLI has
//! always printed, and declarations matched back to the files they were
//! emitted for. Nothing TypeScript-shaped leaves the engine.

mod declarations;
mod report;
mod translate;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use super::projection::{self, Probes, ProjectedDocument};
use super::snapshot::Snapshot;
use crate::AnchorKind;
use crate::analysis::{DeclaredVariant, PayloadAlphabet};
use crate::typescript::backend::{Answers, Diagnostic as TsDiagnostic, Resolution, TypeMismatch};
use crate::typescript::mapper::DiagnosticOrigin;

use declarations::*;
pub(crate) use declarations::{externs_of, match_declarations};
pub(crate) use report::report;
use translate::*;
pub(crate) use translate::{name_types, translate, translation_class};

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
    /// Why the TypeScript layer could not answer, classified as unavailable
    /// infrastructure or an internal backend contract failure. The tt-level
    /// diagnostics above are still complete: a missing type checker removes
    /// the *typed* facts, not tt's own answers
    /// (`docs/design/compiler-core.md` §7).
    pub backend_error: Option<BackendError>,
}

/// The public, TypeScript-free classification of a typed-layer failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendError {
    /// Stable classification for consumer presentation and exit policy.
    pub kind: BackendErrorKind,
    /// Human-readable detail from the backend boundary.
    pub message: String,
}

/// Whether a typed-layer failure is environmental or an internal compiler
/// failure. Consumers must not infer this from message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendErrorKind {
    /// A compatible TypeScript toolchain or process cannot be started.
    Unavailable,
    /// A running compiler backend violated its execution or protocol contract.
    Internal,
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(&self.message)
    }
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
