//! The engine's language-service surface — editor semantics in tt's terms.
//!
//! Every question here arrives as a position **in an `.tt` source** and is
//! answered in the same coordinates: hover a value inside a `match` arm and
//! the range that comes back is in the file the user is looking at, never
//! in the TypeScript it lowered to. The whole journey — projection, the
//! byte-exact mappings, serving lowered modules to the TypeScript language
//! service, the completion probe for a construct the user has not finished
//! typing — happens inside the engine. A consumer (the VSCode adapter, or
//! anything else) converts these results to its protocol and nothing more.
//!
//! The TypeScript reach is [`crate::typescript::service`] (`tsgo --lsp`),
//! chosen feature by feature because the API server does not carry the
//! language-service surface yet — an implementation detail this module
//! hides completely (see `docs/design/lsp-architecture.md`).
//!
//! Projections here use [`crate::emit_mapped`], not the typed pipeline's
//! projection: a buffer mid-edit must still project (emit-map is
//! infallible), because the moment completion matters most is the moment
//! the buffer does not compile.

mod project;
mod service;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::project::Project;
use super::projection::{self, module_path_of};
use crate::EmitMapping;
use crate::typescript::mapper;
use crate::typescript::service::{Service, file_uri, service_binary, uri_path};

/// A position in a document: zero-based line, UTF-16 code units — the LSP
/// convention, so an adapter converts nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// Zero-based line.
    pub line: u32,
    /// Zero-based character offset on the line, in UTF-16 code units.
    pub character: u32,
}

/// A range in a document, `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    /// Where the range starts.
    pub start: Position,
    /// Where it ends (exclusive).
    pub end: Position,
}

/// A place in a file the user can open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// The file — an `.tt` source, or a hand-written TypeScript file.
    pub path: PathBuf,
    /// The range within it, in that file's own text.
    pub range: Range,
}

/// One reference to a symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// Where.
    pub location: Location,
    /// Whether this is the declaration. (The service does not mark it; the
    /// first result stands in, as it always has — presentation only.)
    pub is_definition: bool,
}

/// What hover shows: a code-ish signature, optional prose, and the span the
/// answer covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverInfo {
    /// The signature, as the checker displays it.
    pub signature: String,
    /// Documentation prose, empty when there is none.
    pub documentation: String,
    /// The span the hover applies to, in the `.tt` source.
    pub range: Range,
}

/// One completion entry, in the raw terms the adapter ranks and renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    /// The name offered.
    pub label: String,
    /// The service's element kind, normalized to the same strings the
    /// editor has always mapped ("function", "method", "property", ...).
    pub kind: String,
    /// The service's own sort text (the adapter adds its layer prefix).
    pub sort_text: String,
}

/// A completion answer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompletionAnswer {
    /// The entries.
    pub items: Vec<CompletionItem>,
    /// Whether the service answered as a *member* completion — what tells a
    /// real member list from the global scope offered while recovering from
    /// unfinished tt syntax.
    pub member: bool,
    /// Set when the answer came from a completion probe; pass it back to
    /// [`Project::completion_resolve`] so the entry is resolved against the
    /// same probed text it was listed from.
    pub probe: Option<u64>,
}

/// The signature and documentation behind one completion entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionDetail {
    /// The declaration as the checker displays it.
    pub signature: String,
    /// The entry's JSDoc, empty when it has none.
    pub documentation: String,
}

/// One edit of a rename, in the target file's own coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameEdit {
    /// Where the edit applies.
    pub location: Location,
    /// What the service wants written there, with [`RENAME_PLACEHOLDER`]
    /// standing in for the new name — a destructuring shorthand (what a tt
    /// pattern binding compiles to) expands to `field: <placeholder>`, and
    /// dropping that expansion would silently rebind a different field.
    /// `None` means the bare new name.
    pub new_text: Option<String>,
}

/// The name a rename asks the service for, so every edit's text can be read
/// as "the new name, in whatever shape this location needs it".
pub const RENAME_PLACEHOLDER: &str = "ttRenamePlaceholder";

/// One overload in a signature-help answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    /// The rendered signature.
    pub label: String,
    /// Its documentation, empty when there is none.
    pub documentation: String,
    /// The parameters, as `[start, end)` spans into `label`.
    pub parameters: Vec<SignatureParameter>,
}

/// One parameter of a [`Signature`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureParameter {
    /// `[start, end)` of the parameter inside the signature label.
    pub label: (u32, u32),
    /// The parameter's documentation, empty when there is none.
    pub documentation: String,
}

/// Signature help at a call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    /// The overloads.
    pub signatures: Vec<Signature>,
    /// Which overload the cursor is in.
    pub active_signature: u32,
    /// Which parameter the cursor is at.
    pub active_parameter: u32,
}

/// One TypeScript diagnostic, mapped onto the `.tt` source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDiagnostic {
    /// Where, in the `.tt` source.
    pub range: Range,
    /// The message, verbatim.
    pub message: String,
    /// TypeScript's error number, 0 when it had none.
    pub code: u32,
    /// True for a warning; everything else reported here is an error.
    pub warning: bool,
    /// Secondary places this diagnostic points at, each with its own words
    /// — served to the editor as LSP related information. Empty when the
    /// diagnostic has only its primary range.
    pub related: Vec<ServiceRelated>,
}

/// One secondary span of a [`ServiceDiagnostic`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRelated {
    /// The file the span is in, when it is not the diagnostic's own.
    pub path: Option<PathBuf>,
    /// Where, in that file's source.
    pub range: Range,
    /// What this place explains.
    pub message: String,
}

/// The live language-service half of a [`Project`]: the running `tsgo --lsp`
/// conversation and everything served into it.
#[derive(Debug)]
pub(crate) struct ServiceSession {
    client: Service,
    /// The text last served for each `.tt` file — the emitted TypeScript,
    /// or a probe standing in for it.
    served: HashMap<PathBuf, String>,
    /// Unprojected host buffers served at their authored paths.
    host_served: HashMap<PathBuf, String>,
    /// Service projections by source path, reused while the text matches.
    docs: HashMap<PathBuf, Arc<ServiceDoc>>,
    /// The raw items of the last completion answer, so one can be resolved
    /// later: the server resolves the item it produced, not a name. Keyed
    /// by (file, asked offset, label).
    last_completion: HashMap<(PathBuf, usize, String), serde_json::Value>,
    /// The probe the last completion list was answered from, kept so
    /// resolving one of its items can install it again.
    last_probe: Option<ProbeDoc>,
    probe_count: u64,
}

/// One file's language-service projection: the source as it stands (open
/// buffer or disk), the TypeScript it emits, and the byte mappings between
/// them. Unlike the typed pipeline's projection this never fails — a buffer
/// mid-edit still projects, by the emit-map contract.
#[derive(Debug)]
pub(crate) struct ServiceDoc {
    source: String,
    code: String,
    mappings: Vec<EmitMapping>,
    /// The glue each construct wrote — what a diagnostic landing outside
    /// every mapping is *about* (`crate::EmitAnchor`).
    anchors: Vec<crate::EmitAnchor>,
    /// Parser-owned error ranges replaced only in this service projection.
    /// TypeScript diagnostics intersecting one are recovery cascades.
    recovered: Vec<(usize, usize)>,
    /// Direct TT causes found while building this exact projection. The
    /// quick checker layer uses their syntax owners before VSCode ever sees
    /// a provisional consequence.
    tt_diagnostics: Vec<crate::Diagnostic>,
}

/// A compiled completion probe: the buffer with `$tt_probe` spliced in at
/// the cursor, emitted, and the placeholder's position in that output.
#[derive(Debug, Clone)]
struct ProbeDoc {
    path: PathBuf,
    code: String,
    /// UTF-16 offset of the placeholder in `code` — where the service is
    /// asked.
    offset: usize,
    version: u64,
}

/// Inserted at the cursor to complete the construct being typed. `$`-led so
/// it cannot collide with the name the user is in the middle of typing.
const PROBE_NAME: &str = "$tt_probe";

use service::*;
pub(super) use service::{analyses_for, externs_from, externs_of, source_byte, span_range};
