//! A snapshot — the project at one moment, immutable.
//!
//! Every semantic request runs against a snapshot, so an answer is always
//! about one consistent state of the project: the file set, each file's
//! text, and each file's projection, as they were when the snapshot was
//! taken. A later edit produces a *new* snapshot (see
//! [`super::Project::update`]); it never changes this one. That is how
//! stale-answer bugs are ruled out by structure rather than by care.

use std::sync::Arc;

use super::projection::ProjectedDocument;

/// One source that belongs to the snapshot but could not be lowered.
/// Its source and tt diagnostics remain available even though it cannot
/// join the TypeScript program.
#[derive(Debug)]
pub(crate) struct BlockedFile {
    pub(crate) source_path: std::path::PathBuf,
    pub(crate) source: String,
    pub(crate) diagnostics: Vec<crate::Diagnostic>,
    enum_symbols: std::sync::OnceLock<Vec<crate::EnumSymbol>>,
}

impl BlockedFile {
    pub(crate) fn new(
        source_path: std::path::PathBuf,
        source: String,
        diagnostics: Vec<crate::Diagnostic>,
    ) -> Self {
        Self {
            source_path,
            source,
            diagnostics,
            enum_symbols: std::sync::OnceLock::new(),
        }
    }

    pub(crate) fn enum_symbols(&self) -> &[crate::EnumSymbol] {
        self.enum_symbols.get_or_init(|| {
            crate::enum_symbols_with_kind(
                &self.source,
                crate::SourceKind::from_path(&self.source_path).unwrap_or_default(),
            )
        })
    }
}

/// The project at one moment. Cheap to hold: files are shared with the
/// project's cache, so an unchanged file costs one reference.
#[derive(Debug)]
pub struct Snapshot {
    pub(crate) id: u64,
    pub(crate) files: Vec<Arc<ProjectedDocument>>,
    pub(crate) blocked: Vec<Arc<BlockedFile>>,
}

impl Snapshot {
    /// This snapshot's sequence number within its project. Monotonic: a
    /// later snapshot of the same project has a larger id.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The project's `.tt` files, projected, in project order.
    pub fn files(&self) -> &[Arc<ProjectedDocument>] {
        &self.files
    }

    pub(crate) fn blocked(&self) -> &[Arc<BlockedFile>] {
        &self.blocked
    }

    /// The text a diagnostic in `path` was reported against.
    ///
    /// This is the buffer the check actually ran on — an `--overlay`'s
    /// unsaved text included — not whatever is on disk now, so a consumer
    /// that quotes the source cannot draw a caret against a line the
    /// compiler never saw. Covers files that could not be lowered as well
    /// as projected ones: a file is at its most worth quoting exactly when
    /// it is too broken to project.
    ///
    /// `None` when this snapshot does not hold the file — a diagnostic
    /// TypeScript reported in a hand-written `.ts` outside the tt sources.
    pub fn source_of(&self, path: &std::path::Path) -> Option<&str> {
        self.files
            .iter()
            .find(|file| file.source_path == path)
            .map(|file| file.source.as_str())
            .or_else(|| {
                self.blocked
                    .iter()
                    .find(|file| file.source_path == path)
                    .map(|file| file.source.as_str())
            })
    }
}
