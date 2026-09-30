//! The workspace — every project a consumer holds open, answering together.
//!
//! One editor window can hold files of several `tsconfig.json` projects at
//! once: a solution whose `pb` references `pa`, or a `test/` project beside
//! `src/`. Each file's own questions (hover, completion, diagnostics) are
//! answered by the project it belongs to. A question whose answer is
//! spread across projects — every reference to a declaration, every edit a
//! rename makes — is not: `pa` cannot see the uses `pb` makes of it.
//!
//! TypeScript's project service answers those by searching every loaded
//! project that can see the symbol (`getPerProjectReferences`): the
//! requesting file's default project at the requested position, then each
//! other project whose program contains the definition that answer names,
//! at that definition — or, failing that, contains the requesting file, at
//! the requested position — with the combined results deduplicated. A
//! [`Workspace`] is that collection for tt, and [`Workspace::references`]
//! and [`Workspace::rename`] are that search; [`Project::sees`] is its
//! `containsFile`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::documents::Texts;
use super::language::{Location, Position, RENAME_PLACEHOLDER, Reference, RenameEdit};
use super::names::same_location;
use super::{
    Engine, Project, ProjectOptions, TtCompletion, TtDeclarations, TtHint, TtSymbol,
    normalize_document_path,
};

/// A project's identity: the `(tsconfig, root)` pair it was opened as.
pub type ProjectIdentity = (Option<PathBuf>, PathBuf);

/// Every open project, keyed by identity, and the documents held open in
/// them.
#[derive(Debug)]
pub struct Workspace {
    engine: Engine,
    projects: HashMap<ProjectIdentity, Project>,
    /// The documents a consumer holds open, and the project each landed in.
    open: HashMap<PathBuf, ProjectIdentity>,
}

impl Workspace {
    /// An empty workspace whose projects `engine` opens.
    pub fn new(engine: Engine) -> Workspace {
        Workspace {
            engine,
            projects: HashMap::new(),
            open: HashMap::new(),
        }
    }

    /// Holds `text` open for `path` in the project it belongs to, opening
    /// that project on first use. Opening an open document replaces its
    /// text.
    pub fn open_document(&mut self, path: &Path, text: String) -> Result<(), String> {
        let document = normalize_document_path(path)?;
        let identity = Engine::document_project_identity(&document, &ProjectOptions::default())?;
        self.project_of(&document, &identity)?
            .open_document(document.clone(), text);
        self.open.insert(document, identity);
        Ok(())
    }

    /// Releases an open document: its file's text is the disk's again.
    pub fn close_document(&mut self, path: &Path) {
        let document = normalize_document_path(path).unwrap_or_else(|_| path.to_path_buf());
        if let Some(identity) = self.open.remove(&document)
            && let Some(project) = self.projects.get_mut(&identity)
        {
            project.close_document(&document);
        }
    }

    /// Whether `path` is held open.
    pub fn is_open(&self, path: &Path) -> bool {
        normalize_document_path(path).is_ok_and(|document| self.open.contains_key(&document))
    }

    /// Releases every project and open document. A consumer replays the
    /// documents it holds before asking again.
    pub fn reload(&mut self) {
        self.projects.clear();
        self.open.clear();
        self.engine.documents.clear();
    }

    /// [`super::tt_symbol_at`], with the files `source` imports read as
    /// this workspace holds them open.
    pub fn tt_symbol_at(&self, path: &Path, source: &str, position: Position) -> Option<TtSymbol> {
        super::names::symbol_at(path, source, position, self.texts())
    }

    /// [`super::tt_completions_at`], with the files `source` imports read
    /// as this workspace holds them open.
    pub fn tt_completions_at(
        &self,
        path: &Path,
        source: &str,
        position: Position,
    ) -> Vec<TtCompletion> {
        super::completions::completions_at(path, source, position, self.texts())
    }

    /// Whether `position` is a pattern position tt completes.
    pub fn is_pattern_position(&self, path: &Path, source: &str, position: Position) -> bool {
        super::completions::pattern_question(path, source, position, self.texts()).is_some()
    }

    /// [`super::tt_hints`], with the files `source` imports read as this
    /// workspace holds them open.
    pub fn tt_hints(&self, path: &Path, source: &str) -> Vec<TtHint> {
        super::hints::hints(path, source, self.texts())
    }

    /// [`super::tt_declarations`], with the files `source` imports read as
    /// this workspace holds them open.
    pub fn tt_declarations(&self, path: &Path, source: &str) -> TtDeclarations {
        super::declarations::declarations(path, source, self.texts())
    }

    fn texts(&self) -> Texts<'_> {
        Texts::Open(&self.engine.documents)
    }

    /// The project `path` belongs to — the one it was opened in, or the one
    /// its identity names, opened on first use.
    pub fn project_for(&mut self, path: &Path) -> Result<&mut Project, String> {
        let identity = self.identity_for(path)?;
        let document = normalize_document_path(path)?;
        self.project_of(&document, &identity)
    }

    /// Every reference to the symbol at `position`, from every open
    /// project that can see it.
    ///
    /// The requesting file's project answers first; its definition of the
    /// symbol is then asked in each other open project that sees it, and
    /// the answers are merged — a place two projects both report is one
    /// reference, a definition when either says so.
    pub fn references(
        &mut self,
        path: &Path,
        position: Position,
    ) -> Result<Vec<Reference>, String> {
        let default = self.identity_for(path)?;
        let mut references = self.project_for(path)?.references(path, position)?;
        let anchor = references
            .iter()
            .find(|reference| reference.is_definition)
            .map(|reference| reference.location.clone());
        for (identity, at, from) in self.searched(&default, anchor.as_ref(), path, position)? {
            let project = self.projects.get_mut(&identity).expect("listed");
            for reference in project.references(&at, from)? {
                match references
                    .iter_mut()
                    .find(|known| same_location(&known.location, &reference.location))
                {
                    Some(known) => known.is_definition |= reference.is_definition,
                    None => references.push(reference),
                }
            }
        }
        Ok(references)
    }

    /// Every edit renaming the symbol at `position`, from every open
    /// project that can see it — or `Ok(None)` when the rename cannot be
    /// done whole.
    ///
    /// The requesting file's project decides what is renamed. When its
    /// edits rename the symbol's own declaration, every other open project
    /// that sees it is asked to rename at that declaration; when they stop
    /// at a local alias (`import { mk }` renamed as `mk as next`), the
    /// declaration is not renamed and only the projects that see the
    /// requesting file are asked, where the request was.
    /// Any project that refuses, or two edits that disagree about one
    /// place, refuse the whole rename: a program renamed by halves is
    /// corrupted, not renamed.
    pub fn rename(
        &mut self,
        path: &Path,
        position: Position,
    ) -> Result<Option<Vec<RenameEdit>>, String> {
        let default = self.identity_for(path)?;
        let project = self.project_for(path)?;
        let Some(mut edits) = project.rename(path, position)? else {
            return Ok(None);
        };
        let anchor = project
            .definition(path, position)?
            .into_iter()
            .find(|definition| {
                edits
                    .iter()
                    .any(|edit| same_location(&edit.location, definition))
            });
        for (identity, at, from) in self.searched(&default, anchor.as_ref(), path, position)? {
            let project = self.projects.get_mut(&identity).expect("listed");
            let Some(found) = project.rename(&at, from)? else {
                return Ok(None);
            };
            for edit in found {
                match edits
                    .iter()
                    .find(|known| overlaps(&known.location, &edit.location))
                {
                    Some(known)
                        if same_location(&known.location, &edit.location)
                            && written(known) == written(&edit) => {}
                    Some(_) => return Ok(None),
                    None => edits.push(edit),
                }
            }
        }
        Ok(Some(edits))
    }

    /// The range a rename at `position` replaces in `path`, when the rename
    /// can be done whole: the edit of [`Workspace::rename`] that covers the
    /// position. Refused, with TypeScript's reason when it gave one, when
    /// the rename is refused or covers nothing there.
    pub fn prepare_rename(
        &mut self,
        path: &Path,
        position: Position,
    ) -> Result<super::language::PrepareRename, String> {
        use super::language::PrepareRename;
        let document = normalize_document_path(path)?;
        if let Err(reason) = self.project_for(path)?.rename_answer(path, position)? {
            return Ok(PrepareRename::Refused(reason));
        }
        let Some(edits) = self.rename(path, position)? else {
            return Ok(PrepareRename::Refused(None));
        };
        let before = |a: Position, b: Position| (a.line, a.character) <= (b.line, b.character);
        Ok(edits
            .into_iter()
            .map(|edit| edit.location)
            .find(|location| {
                normalize_document_path(&location.path).is_ok_and(|found| found == document)
                    && before(location.range.start, position)
                    && before(position, location.range.end)
            })
            .map_or(PrepareRename::Refused(None), |location| {
                PrepareRename::Range(location.range)
            }))
    }

    /// The identity `path` answers under: the project its open document
    /// landed in, else the one its location names.
    fn identity_for(&self, path: &Path) -> Result<ProjectIdentity, String> {
        let document = normalize_document_path(path)?;
        match self.open.get(&document) {
            Some(identity) => Ok(identity.clone()),
            None => Engine::document_project_identity(&document, &ProjectOptions::default()),
        }
    }

    /// The project `identity` names, opened for `document` on first use.
    fn project_of(
        &mut self,
        document: &Path,
        identity: &ProjectIdentity,
    ) -> Result<&mut Project, String> {
        match self.projects.entry(identity.clone()) {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.into_mut()),
            std::collections::hash_map::Entry::Vacant(entry) => Ok(entry.insert(
                self.engine
                    .open_document_project(document, &ProjectOptions::default())?,
            )),
        }
    }

    /// Where each open project but `default` is asked: at `anchor` when
    /// it sees the anchor's file, else at the request when it sees the
    /// requesting file. A project that sees neither cannot see the symbol
    /// and is not asked. The order is stable, so a combined answer lists
    /// its places the same way each time.
    fn searched(
        &mut self,
        default: &ProjectIdentity,
        anchor: Option<&Location>,
        path: &Path,
        position: Position,
    ) -> Result<Vec<(ProjectIdentity, PathBuf, Position)>, String> {
        let mut others: Vec<_> = self
            .projects
            .keys()
            .filter(|identity| *identity != default)
            .cloned()
            .collect();
        others.sort();
        let mut searched = Vec::new();
        for identity in others {
            let project = self.projects.get_mut(&identity).expect("listed");
            if let Some(anchor) = anchor
                && project.sees(&anchor.path)?
            {
                searched.push((identity, anchor.path.clone(), anchor.range.start));
            } else if project.sees(path)? {
                searched.push((identity, path.to_path_buf(), position));
            }
        }
        Ok(searched)
    }
}

/// What an edit writes, with the bare new name spelled as the placeholder.
fn written(edit: &RenameEdit) -> &str {
    edit.new_text.as_deref().unwrap_or(RENAME_PLACEHOLDER)
}

/// Whether two locations touch the same text of the same file.
fn overlaps(a: &Location, b: &Location) -> bool {
    let key = |position: Position| (position.line, position.character);
    let file = |path: &Path| normalize_document_path(path).unwrap_or_else(|_| path.to_path_buf());
    file(&a.path) == file(&b.path)
        && (a.range == b.range
            || (key(a.range.start) < key(b.range.end) && key(b.range.start) < key(a.range.end)))
}
