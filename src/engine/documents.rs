//! The documents a consumer holds open — one store for every project.
//!
//! An unsaved buffer is a fact about a file, not about the project that
//! was asked about it first: when `test/b.tt` imports `../src/a.tt`, the
//! text of the open `a.tt` is what `test`'s project has to compile too.
//! typescript-go keeps its overlay filesystem on the session for the same
//! reason, and every project reads through it
//! (`docs/design/engine-architecture.md` §B.2). Here the [`super::Engine`]
//! owns the store and hands it to each project it opens.
//!
//! Nothing is invalidated when the store changes. Each project compares a
//! file's current text with the text its caches were built from when it
//! next takes a snapshot or serves a file, so an edit reaches every
//! dependent in every project at its next question.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard};

/// Unsaved text standing in for files on disk, keyed by canonical path.
#[derive(Debug, Clone, Default)]
pub(crate) struct Documents(Arc<RwLock<HashMap<PathBuf, String>>>);

impl Documents {
    /// Every open document's current text.
    pub(crate) fn read(&self) -> RwLockReadGuard<'_, HashMap<PathBuf, String>> {
        self.0.read().unwrap_or_else(PoisonError::into_inner)
    }

    /// Makes `text` the current text of `path`.
    pub(crate) fn set(&self, path: PathBuf, text: String) {
        self.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(path, text);
    }

    /// Returns `path` to its text on disk.
    pub(crate) fn remove(&self, path: &Path) {
        self.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(path);
    }

    /// Returns every file to its text on disk.
    pub(crate) fn clear(&self) {
        self.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }
}
