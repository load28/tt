//! The project — the authoritative, long-lived state of one workspace.
//!
//! A [`Project`] owns everything a semantic answer depends on: which files
//! are in the graph, what each one's current text is (disk or overlay), each
//! file's cached projection, and the running TypeScript session. Consumers
//! never talk to the compiler behind it — they take a [`Snapshot`] and ask
//! about that.
//!
//! The lifecycle mirrors typescript-go's project service, sized to tt:
//! mutation happens on the project (documents open, change, close; disk
//! moves), and [`Project::update`] is the single funnel that turns the
//! current state into an immutable [`Snapshot`]. A file whose text is
//! unchanged between two snapshots keeps its projection — that is the
//! engine's incrementality, and it composes with the session's own (the
//! compiler process stays up and only changed modules are re-served).

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::documents::Documents;
use super::projection::{self, ProjectedDocument};
use super::semantics::{self, Checked, FileSemantics};
use super::snapshot::Snapshot;
use crate::CompileError;
use crate::typescript::backend::{FailureKind, TypeScriptBackend};
use crate::typescript::native::NativeBackend;

/// What counts as a tt source, and what counts as hand-written TypeScript.
const TT_EXTENSIONS: &[&str] = &["tt", "ttx"];
pub(super) const TS_EXTENSIONS: &[&str] = &["ts", "tsx", "mts", "cts"];

/// A snapshot could not be taken because a source could not be read.
/// Lowering failures are recoverable snapshot data; an I/O failure has no
/// source text to preserve and remains a pass-level failure.
#[derive(Debug)]
pub struct Blocked {
    /// The file that failed to lower.
    pub path: PathBuf,
    /// Its tt-level error, with the file's own position.
    pub error: CompileError,
}

/// What one check is asked for, beside the snapshot.
#[derive(Debug, Clone, Copy, Default)]
pub struct CheckRequest {
    /// Emit declarations and return them (`--types`). A plain check does not.
    pub emit_declarations: bool,
    /// Report only the tt layer: every diagnostic of a tt rule, the ones
    /// the checker's answers decide included, and none of TypeScript's own.
    /// The type layer is TypeScript's answer about the user's own code, and
    /// a caller that already has it from somewhere else (an editor with a
    /// live language server) would show it twice.
    pub tt_only: bool,
}

/// The paths a compile depends on, split as a build integration watches
/// them: a file by its content, a directory by the entries it lists.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dependencies {
    /// Files whose content the compile reads, sorted, and the configuration
    /// paths whose absence it relies on.
    pub files: Vec<PathBuf>,
    /// Directories whose entries the compile lists, sorted.
    pub directories: Vec<PathBuf>,
}

impl Dependencies {
    /// The JSON `--dependencies` prints and the server's `dependencies`
    /// answers: `{ "files": [...], "directories": [...] }`.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "files": self.files, "directories": self.directories })
    }
}

/// One workspace's compiler state: documents, projections, and the session.
#[derive(Debug)]
pub struct Project {
    pub(crate) root: PathBuf,
    tsconfig: Option<PathBuf>,
    /// Whether `tsconfig` was found by discovery from the inputs rather
    /// than named.
    pub(super) discovers_config: bool,
    /// The output tree a scan must not descend into (`--types`'s sidecar
    /// directory).
    out_dir: Option<PathBuf>,
    /// The inputs' `.tt` files — what a `--types` run writes. The TypeScript
    /// program owns graph membership; this only narrows emission.
    requested: HashSet<PathBuf>,
    pub(super) input_roots: Vec<PathBuf>,
    pub(super) named: Vec<PathBuf>,
    dependencies: RefCell<HashSet<PathBuf>>,
    /// Directories the compiler listed while resolving the program.
    directories: RefCell<HashSet<PathBuf>>,
    /// The sources the last check's TypeScript programs contained, or
    /// `None` when that check had no TypeScript program to leave one out.
    members: RefCell<Option<HashSet<PathBuf>>>,
    /// Candidate files for the first layered-filesystem pass, fixed at open:
    /// the project scan together with the inputs the caller named. The
    /// configured TypeScript program filters these to actual members.
    initial: Vec<PathBuf>,
    /// The project's hand-written TypeScript, listed only when there is no
    /// `tsconfig.json` to decide the program's files — see
    /// [`crate::typescript::backend::Query::sources`].
    sources: Vec<PathBuf>,
    /// Unsaved text standing in for files on disk, keyed by canonical path
    /// — the engine's store, shared with every project it opened.
    pub(crate) overlays: Documents,
    /// The documents opened through this project: the files that are roots
    /// by request here, whatever the configuration includes.
    pub(super) opened: HashSet<PathBuf>,
    /// Projections by path, kept across snapshots. An entry is reused when
    /// the file's current text equals the projected text.
    cache: HashMap<PathBuf, Arc<ProjectedDocument>>,
    /// The TypeScript backend — or why there is none (no toolchain found).
    /// A project without one still opens and still answers the tt layer;
    /// only the typed facts degrade to unknown ([`Project::check`]).
    backend: Result<NativeBackend, String>,
    /// Cross-snapshot semantic cache, keyed per file by (content hash,
    /// imported declarations). A change to a dependency's body leaves an
    /// importer's entry valid; a change to its exported declarations
    /// invalidates exactly the importers — the invalidation boundary of
    /// `docs/design/compiler-core.md` §11.
    pattern_analysis_cache: RefCell<HashMap<PathBuf, CachedPatternAnalysis>>,
    /// How many per-file semantic computations the cache answered without
    /// recomputing, over the project's lifetime — observability for the
    /// invalidation contract (and its tests).
    pattern_analysis_cache_hits: Cell<usize>,
    /// The last contextual materialization, reused by the next snapshot
    /// that asks the same question over the same disk generation: a request
    /// on an unchanged project asks the checker nothing, as a language
    /// service reuses its program while the project version is unchanged.
    materialized: RefCell<Option<Materialized>>,
    /// Each module's materialization from a run scoped to one file's
    /// reference closure ([`Project::update_scoped`]), kept while the inputs
    /// of the closure it was computed in are unchanged.
    scoped: RefCell<HashMap<PathBuf, ScopedMaterialization>>,
    /// Each file's last reference closure and what it was computed from.
    closures: RefCell<HashMap<PathBuf, KnownClosure>>,
    next_snapshot: u64,
    /// The language-service half — the running `tsgo --lsp` conversation —
    /// started by the first editor question ([`crate::engine::language`]).
    pub(crate) service: Option<super::language::ServiceSession>,
}

impl Project {
    pub(crate) fn new(
        root: PathBuf,
        tsconfig: Option<PathBuf>,
        out_dir: Option<PathBuf>,
        collected: Vec<PathBuf>,
        initial: Vec<PathBuf>,
        sources: Vec<PathBuf>,
        backend: Result<NativeBackend, String>,
    ) -> Project {
        Project {
            root,
            tsconfig,
            discovers_config: false,
            out_dir,
            requested: collected.into_iter().collect(),
            input_roots: Vec::new(),
            named: Vec::new(),
            dependencies: RefCell::new(HashSet::new()),
            directories: RefCell::new(HashSet::new()),
            members: RefCell::new(None),
            initial,
            sources,
            overlays: Documents::default(),
            opened: HashSet::new(),
            cache: HashMap::new(),
            backend,
            pattern_analysis_cache: RefCell::new(HashMap::new()),
            pattern_analysis_cache_hits: Cell::new(0),
            materialized: RefCell::new(None),
            scoped: RefCell::new(HashMap::new()),
            closures: RefCell::new(HashMap::new()),
            next_snapshot: 0,
            service: None,
        }
    }

    pub(crate) fn service_arrangement(&self) -> crate::typescript::service::Arrangement {
        crate::typescript::service::Arrangement::of_project(
            self.backend.as_ref().ok(),
            self.tsconfig.as_deref(),
            &self.root,
        )
    }

    /// The project root — the directory the compiler runs in.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The `(tsconfig, root)` pair this project was opened as — what
    /// [`super::Engine::project_identity`] answers for its inputs.
    pub fn identity(&self) -> (Option<&Path>, &Path) {
        (self.tsconfig.as_deref(), &self.root)
    }

    /// The inputs' own `.tt` files: what an emitting pass writes for.
    pub fn requested(&self) -> &HashSet<PathBuf> {
        &self.requested
    }

    /// Substitutes `text` for `path`'s contents on disk, keyed by the
    /// canonical path. This is how an editor has the buffer it is showing
    /// checked as part of the project it belongs to: the module keeps its
    /// real path — so its imports, and the imports that name it, resolve
    /// exactly as they do on disk — and only its text is the unsaved one.
    ///
    /// The text is the file's for every project the same [`super::Engine`]
    /// opened, so a project that imports the file compiles the buffer too;
    /// the document is a root by request only here.
    pub fn open_document(&mut self, path: PathBuf, text: String) {
        self.opened.insert(path.clone());
        self.overlays.set(path, text);
    }

    /// Replaces an open document's text. The next [`Project::update`] sees
    /// the new text; snapshots already taken keep the old one.
    pub fn update_document(&mut self, path: PathBuf, text: String) {
        self.open_document(path, text);
    }

    /// Closes an open document: the file's text is the disk's again, for
    /// every project.
    pub fn close_document(&mut self, path: &Path) {
        self.opened.remove(path);
        self.overlays.remove(path);
    }

    /// Every candidate `.tt` file under the project root, as sorted absolute
    /// paths. TypeScript later decides which candidates are configured or
    /// reachable. Scanned fresh so a newly created file is seen.
    pub fn scan(&self) -> std::io::Result<Vec<PathBuf>> {
        let mut candidates = project_sources(&self.root, self.out_dir.as_deref(), TT_EXTENSIONS)?;
        candidates.extend(self.requested.iter().filter(|file| file.exists()).cloned());
        candidates.sort();
        candidates.dedup();
        Ok(candidates)
    }

    /// Every path whose change invalidates a project check: the files and
    /// directories of [`Project::dependencies`] together.
    pub fn watch_paths(&self) -> std::io::Result<Vec<PathBuf>> {
        let Dependencies {
            mut files,
            directories,
        } = self.dependencies()?;
        files.extend(directories);
        files.sort();
        files.dedup();
        Ok(files)
    }

    /// What a project check depends on: the source, configuration, and
    /// compiler-resolved files it reads, and the directories whose listing
    /// it reads — a file added to or removed from one can change the
    /// program, while the directory is not itself an input.
    ///
    /// A project without a configuration takes its program from the walk
    /// of its root, so the files and directories that walk listed are
    /// dependencies too.
    pub fn dependencies(&self) -> std::io::Result<Dependencies> {
        let (mut files, walked) = if self.tsconfig.is_none() {
            project_tree(
                &self.root,
                self.out_dir.as_deref(),
                &["tt", "ttx", "ts", "tsx", "mts", "cts", "json"],
            )?
        } else {
            (Vec::new(), Vec::new())
        };
        files.extend(self.dependencies.borrow().iter().cloned());
        files.extend(self.requested.iter().cloned());
        files.extend(self.cache.keys().cloned());
        if let Some(config) = &self.tsconfig {
            files.push(config.clone());
        }
        files.sort();
        files.dedup();
        let mut directories: Vec<_> = self.directories.borrow().iter().cloned().collect();
        directories.extend(walked);
        directories.sort();
        directories.dedup();
        Ok(Dependencies { files, directories })
    }

    /// The candidate set the first pass layers, decided when the project was
    /// opened: the project scan and the inputs the caller named. A named
    /// file is a root by request, so it is checked even when the project's
    /// own configuration does not list it.
    pub fn initial_files(&self) -> Vec<PathBuf> {
        self.initial.clone()
    }

    /// Whether `path` is part of what this project compiles, as far as the
    /// engine can tell: a document opened through it, a tt module its
    /// graph reaches from its candidates (taken fresh), or a file its
    /// compiler read in a typed check. A project serves its TypeScript no
    /// tt module outside that graph, so for tt sources this is exactly what
    /// its program can contain — TypeScript's `containsFile`.
    pub fn sees(&mut self, path: &Path) -> Result<bool, String> {
        let path = super::normalize_document_path(path)?;
        if self.opened.contains(&path) || self.dependencies.borrow().contains(&path) {
            return Ok(true);
        }
        let snapshot = self
            .update(&self.initial_files())
            .map_err(|blocked| blocked.error.to_string())?;
        Ok(snapshot.files().iter().any(|file| file.source_path == path)
            || snapshot
                .blocked()
                .iter()
                .any(|file| file.source_path == path))
    }

    /// Takes a snapshot of `files` and their reachable tt imports: overlay text where a
    /// document is open, disk text otherwise. A file whose text is unchanged
    /// since the last snapshot keeps its projection; the rest are
    /// re-projected. A file that cannot lower remains in the snapshot as a
    /// blocked source with its tt diagnostics; other files still project.
    /// An I/O failure still blocks the snapshot because no source state is
    /// available to preserve.
    pub fn update(&mut self, files: &[PathBuf]) -> Result<Snapshot, Box<Blocked>> {
        self.update_scoped(files, None)
    }

    /// [`Project::update`] for questions about one file. Contextual storage
    /// types are settled only for the modules that file's types can depend on
    /// (TypeScript's referenced-file closure); the snapshot's other modules
    /// keep a materialization that is still current, or their lowered text.
    /// Their own questions settle them in turn. A module's materialization
    /// depends only on its closure, so a scoped run settles every module of
    /// the closure exactly as a whole-project run does.
    pub fn update_scoped(
        &mut self,
        files: &[PathBuf],
        target: Option<&Path>,
    ) -> Result<Snapshot, Box<Blocked>> {
        self.requested.extend(
            files
                .iter()
                .filter(|file| self.input_roots.iter().any(|root| file.starts_with(root)))
                .cloned(),
        );
        if self.tsconfig.is_none() {
            self.sources = project_sources(&self.root, self.out_dir.as_deref(), TS_EXTENSIONS)
                .map(|sources| {
                    sources
                        .into_iter()
                        .filter(|source| !crate::ownership::owned_output(source))
                        .collect()
                })
                .map_err(|error| {
                    Box::new(Blocked {
                        path: self.root.clone(),
                        error: CompileError {
                            message: error.to_string(),
                            filename: None,
                            line: 0,
                            col: 0,
                            end_line: 0,
                            end_col: 0,
                        },
                    })
                })?;
        }
        if let Some(unnamed) = self.sources.iter().find(|path| path.to_str().is_none()) {
            return Err(unnameable(unnamed));
        }
        let mut projected = Vec::with_capacity(files.len());
        let mut blocked_files = Vec::new();
        let mut cache = HashMap::with_capacity(files.len());
        // Projection already owns each content version's import metadata.
        // Follow those edges here instead of reading and parsing every input
        // once for discovery and again for projection.
        let documents = self.overlays.clone();
        let overlays = documents.read();
        if let Some(unnamed) = overlays.keys().find(|path| path.to_str().is_none()) {
            return Err(unnameable(unnamed));
        }
        let mut pending = files.to_vec();
        let mut seen: HashSet<_> = files.iter().cloned().collect();
        let mut imported = HashSet::new();
        let mut unread = Vec::new();
        let mut cursor = 0;
        while cursor < pending.len() {
            let file = pending[cursor].clone();
            cursor += 1;
            let file = &file;
            if file.to_str().is_none() {
                return Err(unnameable(file));
            }
            let text = match overlays.get(file) {
                Some(text) => text.clone(),
                None => match std::fs::read_to_string(file) {
                    Ok(text) => text,
                    Err(e) => {
                        let failure = Box::new(Blocked {
                            path: file.clone(),
                            error: CompileError {
                                message: format!("cannot read: {e}"),
                                filename: Some(file.display().to_string()),
                                line: 0,
                                col: 0,
                                end_line: 0,
                                end_col: 0,
                            },
                        });
                        // A file the project scan found, which no input
                        // names and nothing imports, is one module the
                        // checker does not get — TypeScript's own scan
                        // skips an entry it cannot read — not a reason the
                        // check cannot run.
                        if self.requested.contains(file) || imported.contains(file) {
                            return Err(failure);
                        }
                        unread.push(failure);
                        continue;
                    }
                },
            };
            let open = self.opened.contains(file);
            let doc = match self.cache.get(file) {
                Some(cached) if cached.source == text && (open || !cached.unparsed) => {
                    Some(cached.clone())
                }
                _ => match crate::ice::working_on(file, || {
                    crate::ice::panic_for_test(&format!(
                        "projection:{}",
                        file.file_name().unwrap_or_default().to_string_lossy()
                    ));
                    ProjectedDocument::project_for_snapshot(file, text, open)
                }) {
                    Ok(doc) => Some(Arc::new(doc)),
                    Err(blocked) => {
                        discover_imports(
                            file,
                            blocked.tt_imports(),
                            &overlays,
                            &mut pending,
                            &mut seen,
                            &mut imported,
                        );
                        blocked_files.push(Arc::new(blocked));
                        None
                    }
                },
            };
            if let Some(doc) = doc {
                discover_imports(
                    file,
                    doc.tt_imports(),
                    &overlays,
                    &mut pending,
                    &mut seen,
                    &mut imported,
                );
                cache.insert(file.clone(), doc.clone());
                projected.push(doc);
            }
        }
        if let Some(failure) = unread
            .into_iter()
            .find(|failure| imported.contains(&failure.path))
        {
            return Err(failure);
        }
        // Entries for files that left the project go with the old map; a
        // blocked update above leaves the previous cache intact instead, so
        // the files that were fine keep their projections.
        self.cache = cache;
        let blocked = |failure: crate::typescript::backend::Failure| {
            Box::new(Blocked {
                path: self.root.clone(),
                error: CompileError {
                    message: failure.message,
                    filename: None,
                    line: 0,
                    col: 0,
                    end_line: 0,
                    end_col: 0,
                },
            })
        };
        // A backend that cannot start leaves the projections unrefined, as a
        // missing toolchain does; `check` reports it as unavailable.
        let available =
            |backend: &NativeBackend| match backend.open(self.tsconfig.as_deref(), &self.root) {
                Ok(()) => Ok(true),
                Err(failure) if failure.kind == FailureKind::Unavailable => Ok(false),
                Err(failure) => Err(blocked(failure)),
            };
        if projected
            .iter()
            .any(|doc| !doc.emit.contextual_slots.is_empty())
            && let Ok(backend) = &self.backend
            && available(backend)?
        {
            let (mut query, _) =
                projection::assemble(&projected, &blocked_files, &self.root, &self.sources);
            query
                .modules
                .retain(|module| !projected.iter().any(|doc| doc.module_path == module.path));
            query.modules.extend(
                overlays
                    .iter()
                    .filter(|(path, _)| is_host_source(path))
                    .map(|(path, text)| crate::typescript::backend::Module {
                        path: path.clone(),
                        text: text.clone(),
                    }),
            );
            let mut order: Vec<usize> = (0..projected.len()).collect();
            order.sort_by(|&left, &right| {
                projected[left]
                    .module_path
                    .cmp(&projected[right].module_path)
            });
            let mut roots = self.roots(&projected, &[]);
            roots.sort();
            query
                .modules
                .sort_by(|left, right| left.path.cmp(&right.path));
            if let Some(target) = target
                && let Some(emits) = self
                    .scoped_emits(backend, &projected, &order, &query, &roots, target)
                    .map_err(blocked)?
            {
                for (index, emit) in emits {
                    if projected[index].emit != emit {
                        Arc::make_mut(&mut projected[index]).replace_emit(emit);
                    }
                }
                self.next_snapshot += 1;
                return Ok(Snapshot {
                    id: self.next_snapshot,
                    files: projected,
                    blocked: blocked_files,
                    host_overlays: overlays
                        .iter()
                        .filter(|(path, _)| is_host_source(path))
                        .map(|(path, text)| (path.clone(), text.clone()))
                        .collect(),
                });
            }
            let read = Arc::new(ServedTexts::new(&projected, &query.modules).0);
            let question = ContextualQuestion {
                modules: order
                    .iter()
                    .map(|&index| {
                        (
                            projected[index].module_path.clone(),
                            projected[index].emit.clone(),
                        )
                    })
                    .collect(),
                support: query.modules,
                sources: query.sources,
                roots,
            };
            let emits = self.contextual_emits(backend, question).map_err(blocked)?;
            // A whole-project materialization settles every module as its
            // closure would: a later question about one file reuses it while
            // no input of the project changes.
            let generation = self
                .materialized
                .borrow()
                .as_ref()
                .map(|materialized| materialized.generation);
            let mut cache = self.scoped.borrow_mut();
            for (&index, emit) in order.iter().zip(emits) {
                if let Some(generation) = generation {
                    cache.insert(
                        projected[index].module_path.clone(),
                        ScopedMaterialization {
                            read: read.clone(),
                            generation,
                            source: source_digest(&projected[index].source),
                            emit: emit.clone(),
                        },
                    );
                }
                if projected[index].emit != emit {
                    Arc::make_mut(&mut projected[index]).replace_emit(emit);
                }
            }
        }
        self.next_snapshot += 1;
        Ok(Snapshot {
            id: self.next_snapshot,
            files: projected,
            blocked: blocked_files,
            host_overlays: overlays
                .iter()
                .filter(|(path, _)| is_host_source(path))
                .map(|(path, text)| (path.clone(), text.clone()))
                .collect(),
        })
    }

    /// The refined emits of the projected modules `target`'s types can
    /// depend on, and the unchanged materializations of the others, by index
    /// into `projected`. `None` when the target is not projected, the backend
    /// cannot name its closure, or the disk changed while it was being
    /// materialized; the whole project is materialized instead.
    fn scoped_emits(
        &self,
        backend: &NativeBackend,
        projected: &[Arc<ProjectedDocument>],
        order: &[usize],
        support: &crate::typescript::backend::Query,
        roots: &[PathBuf],
        target: &Path,
    ) -> Result<Option<Vec<(usize, crate::MappedEmit)>>, crate::typescript::backend::Failure> {
        use crate::typescript::backend::{Module, Query};
        let config = self.tsconfig.as_deref();
        let target = if is_host_source(target) {
            target.to_path_buf()
        } else {
            match projected.iter().find(|doc| doc.source_path == target) {
                Some(doc) => doc.module_path.clone(),
                None => return Ok(None),
            }
        };
        let texts = ServedTexts::new(projected, &support.modules);
        let generation = backend.current_generation(config, &self.root);
        let unchanged = Unchanged::new(&texts, generation);
        // The checker is served each module's unchanged materialization, so
        // the closure is read from those texts too: asking with the lowered
        // ones would make the checker rebuild its program twice.
        let served: Vec<Module> = {
            let cache = self.scoped.borrow();
            support
                .modules
                .iter()
                .cloned()
                .chain(order.iter().map(|&index| {
                    let path = &projected[index].module_path;
                    let text = match cache.get(path) {
                        Some(entry) if unchanged.of(entry) => entry.emit.code.clone(),
                        _ => projected[index].emit.code.clone(),
                    };
                    Module {
                        path: path.clone(),
                        text,
                    }
                }))
                .collect()
        };
        let asked = ClosureQuestion::new(&served, &support.sources, roots, generation);
        let reused = self
            .closures
            .borrow()
            .get(&target)
            .filter(|known| known.question == asked)
            .map(|known| known.members.clone());
        let members = match reused {
            Some(members) => members,
            None => {
                backend.observe_generations();
                let answers = backend.ask(
                    config,
                    &self.root,
                    &Query {
                        contextual_only: true,
                        reference_closure: Some(target.clone()),
                        sources: support.sources.clone(),
                        roots: roots.to_vec(),
                        modules: served,
                        ..Query::default()
                    },
                )?;
                let Some(closure) = answers.reference_closure else {
                    return Ok(None);
                };
                let mut members = closure.files;
                members.sort();
                members.dedup();
                let members = Arc::new(members);
                self.closures.borrow_mut().insert(
                    target,
                    KnownClosure {
                        question: asked,
                        members: members.clone(),
                    },
                );
                members
            }
        };
        let current = |index: usize, entry: &ScopedMaterialization| {
            entry.source == source_digest(&projected[index].source)
                && unchanged.of(entry)
                && members
                    .iter()
                    .filter(|member| texts.0.contains_key(*member))
                    .all(|member| entry.read.contains_key(member))
        };
        let in_closure: Vec<usize> = order
            .iter()
            .copied()
            .filter(|&index| members.binary_search(&projected[index].module_path).is_ok())
            .collect();
        let settled = {
            let cache = self.scoped.borrow();
            in_closure.iter().all(|&index| {
                projected[index].emit.contextual_slots.is_empty()
                    || cache
                        .get(&projected[index].module_path)
                        .is_some_and(|entry| current(index, entry))
            })
        };
        if !settled {
            let mut question: Vec<(PathBuf, crate::MappedEmit)> = in_closure
                .iter()
                .map(|&index| {
                    (
                        projected[index].module_path.clone(),
                        projected[index].emit.clone(),
                    )
                })
                .collect();
            let mut others = support.modules.clone();
            others.extend(
                order
                    .iter()
                    .filter(|index| !in_closure.contains(index))
                    .map(|&index| Module {
                        path: projected[index].module_path.clone(),
                        text: projected[index].emit.code.clone(),
                    }),
            );
            others.sort_by(|left, right| left.path.cmp(&right.path));
            backend.observe_generations();
            crate::typescript::contextual::materialize(
                backend,
                config,
                &self.root,
                &mut question,
                &others,
                &support.sources,
                roots,
            )?;
            // A disk change during the rounds leaves results that are not
            // current for the generation this request read.
            let Some(stable) = backend
                .stable_generation()
                .filter(|stable| Some(*stable) == generation)
            else {
                return Ok(None);
            };
            let read = Arc::new(texts.of(members.iter()));
            let mut cache = self.scoped.borrow_mut();
            for ((path, emit), &index) in question.into_iter().zip(&in_closure) {
                cache.insert(
                    path,
                    ScopedMaterialization {
                        read: read.clone(),
                        generation: stable,
                        source: source_digest(&projected[index].source),
                        emit,
                    },
                );
            }
        }
        let cache = self.scoped.borrow();
        Ok(Some(
            order
                .iter()
                .filter_map(|&index| {
                    let entry = cache.get(&projected[index].module_path)?;
                    // A module outside the closure is not read by questions
                    // about the target; it keeps a materialization whose
                    // served inputs are unchanged.
                    let usable = if in_closure.contains(&index) {
                        current(index, entry)
                    } else {
                        entry.source == source_digest(&projected[index].source)
                            && unchanged.of(entry)
                    };
                    usable.then(|| (index, entry.emit.clone()))
                })
                .collect(),
        ))
    }

    /// The refined emits of `question`'s modules, in its order: the last
    /// materialization's when it answered the same question over the same
    /// host session and disk generation, a new materialization otherwise.
    fn contextual_emits(
        &self,
        backend: &NativeBackend,
        question: ContextualQuestion,
    ) -> Result<Vec<crate::MappedEmit>, crate::typescript::backend::Failure> {
        let config = self.tsconfig.as_deref();
        if let Some(last) = self.materialized.borrow().as_ref()
            && last.question == question
            && backend.current_generation(config, &self.root) == Some(last.generation)
        {
            return Ok(last.emits.clone());
        }
        let mut modules = question.modules.clone();
        backend.observe_generations();
        crate::typescript::contextual::materialize(
            backend,
            config,
            &self.root,
            &mut modules,
            &question.support,
            &question.sources,
            &question.roots,
        )?;
        let emits: Vec<_> = modules.into_iter().map(|(_, emit)| emit).collect();
        *self.materialized.borrow_mut() =
            backend.stable_generation().map(|generation| Materialized {
                question,
                generation,
                emits: emits.clone(),
            });
        Ok(emits)
    }

    /// How many per-file semantic computations the cross-snapshot cache
    /// answered without recomputing. A dependency's body-only change keeps
    /// its importers' entries; an exported-declaration change invalidates
    /// them — this counter is how that contract is observed and tested.
    pub fn semantic_cache_hits(&self) -> usize {
        self.pattern_analysis_cache_hits.get()
    }

    /// The per-file semantics of a snapshot, served from the
    /// cross-snapshot cache where the key — (content, imported
    /// declarations) — still matches.
    fn file_semantics(&self, snapshot: &Snapshot) -> HashMap<PathBuf, Arc<FileSemantics>> {
        let files = snapshot.files();
        let mut out = HashMap::with_capacity(files.len());
        for file in files {
            let externs = semantics::externs_of(snapshot, file);
            let value = self.pattern_analysis(&file.source_path, &file.source, externs);
            out.insert(file.source_path.clone(), value);
        }
        out
    }

    /// One file's semantics, computed only when the cross-snapshot cache
    /// has no entry for this (content, imported declarations) pair — the
    /// single lookup both the typed pass ([`Project::check`]) and the
    /// editor's semantic fallbacks ([`Project::semantic_analyses`]) go
    /// through, so the two surfaces share one cache instead of each
    /// recomputing the other's answer.
    fn pattern_analysis(
        &self,
        path: &Path,
        source: &str,
        externs: Vec<crate::resolve::ImportedVariant>,
    ) -> Arc<FileSemantics> {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        let source_hash = hasher.finish();
        let mut cache = self.pattern_analysis_cache.borrow_mut();
        if let Some(cached) = cache.get(path)
            && cached.source_hash == source_hash
            && cached.value.externs == externs
        {
            self.pattern_analysis_cache_hits
                .set(self.pattern_analysis_cache_hits.get() + 1);
            return cached.value.clone();
        }
        let decls: Vec<crate::resolve::ExternDecl> = externs.iter().map(Into::into).collect();
        let analyses = crate::analysis::pattern_analyses_with_kind(
            source,
            &decls,
            crate::SourceKind::from_path(path).unwrap_or_default(),
        );
        let value = Arc::new(FileSemantics { externs, analyses });
        cache.insert(
            path.to_path_buf(),
            CachedPatternAnalysis {
                source_hash,
                value: value.clone(),
            },
        );
        value
    }

    /// The semantics of one document as the editor sees it (overlay text
    /// first), served from the same cross-snapshot cache as the typed pass.
    /// Imported declarations are read from open overlays, then from the
    /// projection cache (an unchanged import target is not re-parsed), then
    /// from disk.
    pub(crate) fn semantic_analyses(&self, path: &Path, source: &str) -> Arc<FileSemantics> {
        let externs = super::language::externs_from(
            path,
            &crate::tt_imports_with_kind(
                source,
                crate::SourceKind::from_path(path).unwrap_or_default(),
            ),
            &|target| {
                let text = match self.overlays.read().get(target) {
                    Some(text) => text.clone(),
                    None => std::fs::read_to_string(target).ok()?,
                };
                if let Some(doc) = self.cache.get(target)
                    && doc.source == text
                {
                    return Some(doc.exported_variant_symbols().to_vec());
                }
                Some(crate::exported_variant_symbols_with_kind(
                    &text,
                    crate::SourceKind::from_path(target).unwrap_or_default(),
                ))
            },
        );
        self.pattern_analysis(path, source, externs)
    }

    fn roots(&self, files: &[Arc<ProjectedDocument>], requested: &[PathBuf]) -> Vec<PathBuf> {
        files
            .iter()
            .filter(|file| {
                self.named.contains(&file.source_path)
                    || self.opened.contains(&file.source_path)
                    || requested.contains(&file.source_path)
            })
            .map(|file| file.module_path.clone())
            .collect()
    }

    /// The candidates a check of `inputs` covers: the project scan and
    /// every tt source the inputs reach.
    pub fn candidates(&self, inputs: &super::Inputs) -> std::io::Result<Vec<PathBuf>> {
        let mut files = self.scan()?;
        files.extend(inputs.files().iter().cloned());
        files.sort();
        files.dedup();
        Ok(files)
    }

    /// `--dependencies <inputs>`: checks the project with every candidate,
    /// the files the inputs name being roots by request (checked even when
    /// the configuration leaves them out), and answers what that check
    /// depends on. The command line and the server's `dependencies` both
    /// answer through this.
    pub fn dependencies_of(&mut self, inputs: &super::Inputs) -> Result<Dependencies, String> {
        self.check_for_dependencies(inputs)?;
        self.dependencies_for(inputs)
            .map_err(|error| error.to_string())
    }

    /// [`Project::dependencies`], with what deciding `inputs`' project read:
    /// the `tsconfig.json` paths configuration discovery probed for them,
    /// existing or not, as tsserver watches them for an inferred project.
    /// Creating one of them, or deleting the one found, puts the inputs in
    /// another project.
    pub fn dependencies_for(&self, inputs: &super::Inputs) -> std::io::Result<Dependencies> {
        let mut dependencies = self.dependencies()?;
        if self.discovers_config {
            dependencies
                .files
                .extend(tsconfig_lookup(&inputs.collected));
            dependencies.files.sort();
            dependencies.files.dedup();
        }
        Ok(dependencies)
    }

    fn check_for_dependencies(&mut self, inputs: &super::Inputs) -> Result<(), String> {
        let files = self.candidates(inputs).map_err(|error| error.to_string())?;
        let snapshot = self
            .update(&files)
            .map_err(|blocked| blocked.error.message.clone())?;
        let checked =
            self.check_requested(&snapshot, &CheckRequest::default(), &inputs.named, None)?;
        if let Some(error) = checked.backend_error
            && error.kind == super::BackendErrorKind::Internal
        {
            return Err(error.message);
        }
        Ok(())
    }

    /// Whether the last check covered `path`: its TypeScript programs
    /// contained it, or it had none to leave it out of.
    pub fn checked(&self, path: &Path) -> bool {
        self.members
            .borrow()
            .as_ref()
            .is_none_or(|members| members.contains(path))
    }

    /// Checks a snapshot: asks the running compiler about it and returns
    /// diagnostics at `.tt` positions — and the emitted declarations, when
    /// the request wants them. The session persists across calls; only what
    /// changed since the last ask travels.
    pub fn check(&self, snapshot: &Snapshot, request: &CheckRequest) -> Result<Checked, String> {
        self.check_requested(snapshot, request, &[], None)
    }

    /// [`Project::check`] for one file, as a language service checks the
    /// file an editor shows (TypeScript's `semanticCheck(file)` in
    /// `server/session.ts`): TypeScript's diagnostics of that file alone,
    /// whatever another file's syntax, and the tt diagnostics of that file.
    pub fn check_file(
        &self,
        snapshot: &Snapshot,
        request: &CheckRequest,
        file: &Path,
    ) -> Result<Checked, String> {
        self.check_requested(snapshot, request, &[], Some(file))
    }

    /// Ask for editor diagnostics on the exact graph already served by the
    /// language surface. The backend retains projected spans and suggestions.
    pub(super) fn editor_diagnostics(
        &self,
        modules: Vec<crate::typescript::backend::Module>,
        target: PathBuf,
    ) -> Result<Vec<crate::typescript::backend::EditorDiagnostic>, String> {
        let query = crate::typescript::backend::Query {
            modules,
            sources: self.sources.clone(),
            roots: vec![target.clone()],
            editor_diagnostics: Some(target),
            ..Default::default()
        };
        self.backend
            .as_ref()
            .map_err(Clone::clone)?
            .ask(self.tsconfig.as_deref(), &self.root, &query)
            .map(|answer| answer.editor_diagnostics)
            .map_err(|failure| failure.message)
    }

    /// [`Project::check`] with `requested` roots by request besides the
    /// named and open files, for this check only.
    fn check_requested(
        &self,
        snapshot: &Snapshot,
        request: &CheckRequest,
        requested: &[PathBuf],
        scope: Option<&Path>,
    ) -> Result<Checked, String> {
        let semantics = self.file_semantics(snapshot);
        let (mut query, probes) = projection::assemble(
            snapshot.files(),
            snapshot.blocked(),
            &self.root,
            &self.sources,
        );
        query.emit_declarations = request.emit_declarations;
        query.roots = self.roots(snapshot.files(), requested);
        query.diagnostics_scope = scope.map(|file| {
            if is_host_source(file) {
                file.to_path_buf()
            } else {
                snapshot
                    .files()
                    .iter()
                    .find(|projected| projected.source_path == file)
                    .map_or_else(
                        || file.to_path_buf(),
                        |projected| projected.module_path.clone(),
                    )
            }
        });
        query
            .modules
            .extend(snapshot.host_overlays.iter().map(|(path, text)| {
                crate::typescript::backend::Module {
                    path: path.clone(),
                    text: text.clone(),
                }
            }));
        // A backend that cannot run removes the typed facts, not the pass:
        // every typed answer degrades to unknown and the tt layer still
        // reports in full (`docs/design/compiler-core.md` §7).
        let (answers, backend_error) = match &self.backend {
            Ok(backend) => match backend.ask(self.tsconfig.as_deref(), &self.root, &query) {
                Ok(answers) => (answers, None),
                Err(error) => (
                    Default::default(),
                    Some(super::BackendError {
                        kind: match error.kind {
                            FailureKind::Unavailable => super::BackendErrorKind::Unavailable,
                            FailureKind::Internal => super::BackendErrorKind::Internal,
                        },
                        message: error.message,
                    }),
                ),
            },
            Err(missing) => (
                Default::default(),
                Some(super::BackendError {
                    kind: super::BackendErrorKind::Unavailable,
                    message: missing.clone(),
                }),
            ),
        };
        self.dependencies
            .borrow_mut()
            .extend(answers.dependencies.iter().cloned());
        self.directories
            .borrow_mut()
            .extend(answers.directories.iter().cloned());
        *self.members.borrow_mut() = answers.project_modules.as_ref().map(|modules| {
            snapshot
                .files()
                .iter()
                .filter(|file| modules.contains(&file.module_path))
                .map(|file| file.source_path.clone())
                .collect()
        });
        let declarations = if request.emit_declarations && backend_error.is_none() {
            semantics::match_declarations(snapshot, &answers, &self.requested)
        } else {
            Default::default()
        };
        let mut diagnostics = semantics::report(
            snapshot,
            &answers,
            &probes,
            request.tt_only,
            &semantics,
            &self.requested,
        );
        if let Some(file) = scope {
            diagnostics.retain(|diagnostic| diagnostic.path == file);
        }
        Ok(Checked {
            diagnostics,
            declarations,
            backend_error,
        })
    }
}

fn unnameable(path: &Path) -> Box<Blocked> {
    Box::new(Blocked {
        path: path.to_path_buf(),
        error: CompileError {
            message: "path is not valid UTF-8, so TypeScript cannot name this file".to_string(),
            filename: Some(path.display().to_string()),
            line: 0,
            col: 0,
            end_line: 0,
            end_col: 0,
        },
    })
}

/// Host files retain their original paths and syntax in backend overlays.
pub(super) fn is_host_source(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| TS_EXTENSIONS.contains(&extension))
}

/// One cached [`FileSemantics`] with the half of its key the value does
/// not carry (the content hash; the externs are compared on the value).
#[derive(Debug)]
struct CachedPatternAnalysis {
    source_hash: u64,
    value: Arc<FileSemantics>,
}

/// Everything a contextual materialization is asked besides the project's
/// fixed configuration and root, in path order: the projected modules as
/// lowered, the modules served beside them, the listed hand-written
/// sources, and the roots by request.
#[derive(Debug, PartialEq)]
struct ContextualQuestion {
    modules: Vec<(PathBuf, crate::MappedEmit)>,
    support: Vec<crate::typescript::backend::Module>,
    sources: Vec<PathBuf>,
    roots: Vec<PathBuf>,
}

/// One module's materialization: the digest of every served file it read
/// (shared by the modules materialized together), and the backend's disk
/// generation, which covers the files it read from disk.
#[derive(Debug)]
struct ScopedMaterialization {
    read: Arc<std::collections::BTreeMap<PathBuf, u64>>,
    generation: (u64, u64),
    source: u64,
    emit: crate::MappedEmit,
}

fn source_digest(source: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

/// Whether materializations still describe the served texts: the disk is as
/// it was, and every served file one read is still served with the same
/// text. A file no longer served (a closed overlay) is read from disk now,
/// which its materialization did not read. Answers are kept per shared read
/// set, so a request checks each set once.
struct Unchanged<'t> {
    texts: &'t ServedTexts,
    generation: Option<(u64, u64)>,
    known: RefCell<HashMap<*const std::collections::BTreeMap<PathBuf, u64>, bool>>,
}

impl<'t> Unchanged<'t> {
    fn new(texts: &'t ServedTexts, generation: Option<(u64, u64)>) -> Unchanged<'t> {
        Unchanged {
            texts,
            generation,
            known: RefCell::new(HashMap::new()),
        }
    }

    fn of(&self, entry: &ScopedMaterialization) -> bool {
        if Some(entry.generation) != self.generation {
            return false;
        }
        let texts = self.texts;
        *self
            .known
            .borrow_mut()
            .entry(Arc::as_ptr(&entry.read))
            .or_insert_with(|| {
                entry
                    .read
                    .iter()
                    .all(|(path, digest)| texts.0.get(path) == Some(digest))
            })
    }
}

/// A file's reference closure, sorted, and what it was computed from.
#[derive(Debug)]
struct KnownClosure {
    question: ClosureQuestion,
    members: Arc<Vec<PathBuf>>,
}

/// What a reference closure was computed from: the served texts, the
/// checker's sources and roots, and the disk generation.
#[derive(Debug, PartialEq, Eq)]
struct ClosureQuestion {
    texts: u64,
    generation: Option<(u64, u64)>,
}

impl ClosureQuestion {
    fn new(
        served: &[crate::typescript::backend::Module],
        sources: &[PathBuf],
        roots: &[PathBuf],
        generation: Option<(u64, u64)>,
    ) -> ClosureQuestion {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for module in served {
            module.path.hash(&mut hasher);
            module.text.hash(&mut hasher);
        }
        sources.hash(&mut hasher);
        roots.hash(&mut hasher);
        ClosureQuestion {
            texts: hasher.finish(),
            generation,
        }
    }
}

/// The digest of every text the project serves the checker: a lowered
/// module's text before materialization and a support module's text.
struct ServedTexts(std::collections::BTreeMap<PathBuf, u64>);

impl ServedTexts {
    fn new(
        projected: &[Arc<ProjectedDocument>],
        support: &[crate::typescript::backend::Module],
    ) -> ServedTexts {
        use std::hash::{Hash, Hasher};
        let digest = |text: &str| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            text.hash(&mut hasher);
            hasher.finish()
        };
        let mut texts = std::collections::BTreeMap::new();
        for module in support {
            texts.insert(module.path.clone(), digest(&module.text));
        }
        for doc in projected {
            texts.insert(doc.module_path.clone(), digest(&doc.emit.code));
        }
        ServedTexts(texts)
    }

    /// The digests of `files` that the project serves.
    fn of<'p>(
        &self,
        files: impl IntoIterator<Item = &'p PathBuf>,
    ) -> std::collections::BTreeMap<PathBuf, u64> {
        files
            .into_iter()
            .filter_map(|file| self.0.get(file).map(|digest| (file.clone(), *digest)))
            .collect()
    }
}

/// One materialization, kept while its question and the host's disk
/// generation stay the same.
#[derive(Debug)]
struct Materialized {
    question: ContextualQuestion,
    generation: (u64, u64),
    emits: Vec<crate::MappedEmit>,
}

/// Every file of the project with one of `extensions`, as absolute paths.
/// `node_modules`, dot directories and the output tree are skipped — nothing
/// there is a source.
pub(crate) fn project_sources(
    root: &Path,
    out_dir: Option<&Path>,
    extensions: &[&str],
) -> std::io::Result<Vec<PathBuf>> {
    project_tree(root, out_dir, extensions).map(|(files, _)| files)
}

/// [`project_sources`], with the directories the walk listed to find them.
fn project_tree(
    root: &Path,
    out_dir: Option<&Path>,
    extensions: &[&str],
) -> std::io::Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let mut directories = SourceDirectories::new(out_dir);
    let mut files = Vec::new();
    let mut listed = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if !directories.enter(&dir)? {
            continue;
        }
        listed.push(super::paths::canonical(&dir)?);
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if excluded_source_entry(&path) {
                continue;
            }
            // Directory entries already carry the file type on supported
            // filesystems. Only symlinks need a target metadata lookup.
            let kind = entry.file_type()?;
            // A symlink is what its target is. One whose target cannot be
            // read names no file or directory, and TypeScript's directory
            // listing (`getAccessibleFileSystemEntries`) skips it too.
            let kind = if kind.is_symlink() {
                match std::fs::metadata(&path) {
                    Ok(target) => target.file_type(),
                    Err(_) => continue,
                }
            } else {
                kind
            };
            if kind.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| extensions.contains(&e))
            {
                files.push(super::paths::canonical(&path)?);
            }
        }
    }
    files.sort();
    // File symlinks can share an identity even when directories were visited once.
    files.dedup();
    listed.sort();
    Ok((files, listed))
}

/// The nearest `tsconfig.json` at or above the inputs' common directory.
pub(crate) fn find_tsconfig(files: &[PathBuf]) -> Option<PathBuf> {
    tsconfig_lookup(files)
        .pop()
        .filter(|candidate| candidate.is_file())
}

/// Every `tsconfig.json` path [`find_tsconfig`] probes, nearest first, up
/// to the one it finds: creating one of the others, or deleting the last,
/// changes which configuration the inputs belong to.
pub(crate) fn tsconfig_lookup(files: &[PathBuf]) -> Vec<PathBuf> {
    let mut probed = Vec::new();
    let Some(mut dir) = files
        .first()
        .and_then(|file| file.parent())
        .map(Path::to_path_buf)
    else {
        return probed;
    };
    while !files.iter().all(|file| file.starts_with(&dir)) {
        if !dir.pop() {
            return probed;
        }
    }
    loop {
        let candidate = dir.join("tsconfig.json");
        let found = candidate.is_file();
        probed.push(candidate);
        if found || !dir.pop() {
            return probed;
        }
    }
}

/// Collects sources under `entry` the way every ttc mode does: a file is
/// taken as it is; a directory is walked recursively, skipping
/// dot-directories and `node_modules`, taking `.tt` — and, when
/// `include_ts` is set, hand-written TypeScript (`.ts`/`.mts`/`.cts`) too.
/// This enumerator preserves caller-selected roots; output filtering belongs
/// to the build driver. Typed callers collect tt roots only, so emitted
/// `.tt.d.ts`/`.ttx.d.ts` sidecars are not inputs. Project candidate scans
/// independently exclude their configured output tree.
///
/// Files keep the spelling the walk reached them by, and the CLI mirrors
/// that spelling under `-o`. A directory is walked once, under the spelling
/// that reaches it from `entry` without a symlink when one exists; only a
/// directory the link-free walk never reaches is taken through its first
/// alias in sorted order.
pub fn collect_sources(
    entry: &Path,
    include_ts: bool,
    out: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    collect_sources_in(
        entry,
        entry,
        include_ts,
        out,
        &mut SourceDirectories::new(None),
    )
}

fn collect_sources_in(
    root: &Path,
    entry: &Path,
    include_ts: bool,
    out: &mut Vec<PathBuf>,
    directories: &mut SourceDirectories,
) -> std::io::Result<()> {
    let meta = std::fs::metadata(entry).map_err(|e| named(entry, e))?;
    if meta.is_file() {
        // A named file is filtered the same way the walk filters one: the
        // contract is about extensions, not about how the file was reached.
        // Without this, `ttc -o build src/app.js` wrote TypeScript syntax
        // into a file still called `.js`.
        if !is_source(entry, include_ts) {
            return Err(named(
                entry,
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!(
                        "{} (expected {})",
                        if include_ts {
                            "not a tt or TypeScript source"
                        } else {
                            "not a tt source"
                        },
                        source_extensions(include_ts)
                    ),
                ),
            ));
        }
        out.push(entry.to_path_buf());
        return Ok(());
    }
    if meta.is_dir() {
        if !directories.enter(entry)? {
            return Ok(());
        }
        let mut children: Vec<PathBuf> = std::fs::read_dir(entry)
            .map_err(|e| named(entry, e))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<_>>()
            .map_err(|e| named(entry, e))?;
        children.sort();
        for child in children {
            // Exclusion is a property of the entry name, even when its
            // target is missing or unreadable. Inspect only admitted entries.
            if excluded_source_entry(&child) && !is_source(&child, include_ts) {
                continue;
            }
            // A directory holds entries the walk cannot read — a dangling
            // symlink, a loop, a permission. Naming the one that failed is
            // the difference between a fixable report and "the directory
            // you named does not exist", which is what the bare error said
            // about a directory that plainly does.
            let meta = std::fs::metadata(&child).map_err(|e| named(&child, e))?;
            if meta.is_dir() {
                if !excluded_source_entry(&child) && !alias_of_walked_directory(root, &child)? {
                    collect_sources_in(root, &child, include_ts, out, directories)?;
                }
            } else if meta.is_file() && is_source(&child, include_ts) {
                out.push(child);
            }
        }
    }
    Ok(())
}

/// Whether `dir` is a symlink to a directory the walk from `root` also
/// reaches without following one. That directory is walked under its own
/// name, wherever the alias sorts, so which spelling the walk keeps — and
/// where a build mirrors its files — does not depend on how the alias is
/// named. A target outside that link-free walk, or behind an excluded
/// entry, is reached only through its aliases.
fn alias_of_walked_directory(root: &Path, dir: &Path) -> std::io::Result<bool> {
    let is_link = |path: &Path| {
        std::fs::symlink_metadata(path)
            .map(|meta| meta.file_type().is_symlink())
            .map_err(|error| named(path, error))
    };
    if !is_link(dir)? {
        return Ok(false);
    }
    let identity = super::paths::canonical(dir).map_err(|error| named(dir, error))?;
    let base = super::paths::canonical(root).map_err(|error| named(root, error))?;
    let Ok(relative) = identity.strip_prefix(&base) else {
        return Ok(false);
    };
    let mut path = root.to_path_buf();
    for component in relative.components() {
        path.push(component);
        if excluded_source_entry(&path) || is_link(&path)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Exclude generated and vendored entries before probing their targets.
/// Both source walks use the same admission rule for directory children.
fn excluded_source_entry(path: &Path) -> bool {
    path.file_name().is_some_and(|name| {
        let name = name.to_string_lossy();
        name.starts_with('.') || name == "node_modules"
    })
}

/// Directory admission is about filesystem identity, not the spelling of
/// the path used to reach it. Both collectors follow links, so their input
/// is a graph: visit each directory once and exclude output aliases too.
struct SourceDirectories {
    visited: HashSet<PathBuf>,
    excluded: Option<PathBuf>,
}

impl SourceDirectories {
    fn new(out_dir: Option<&Path>) -> Self {
        Self {
            visited: HashSet::new(),
            excluded: out_dir.and_then(|path| super::paths::canonical(path).ok()),
        }
    }

    fn enter(&mut self, path: &Path) -> std::io::Result<bool> {
        // An unreadable input must fail the scan, not silently produce a
        // successful partial build. Optional output identity is different:
        // the output directory may not exist before the first build.
        let identity = super::paths::canonical(path).map_err(|error| named(path, error))?;
        if self
            .excluded
            .as_ref()
            .is_some_and(|excluded| identity.starts_with(excluded))
        {
            return Ok(false);
        }
        Ok(self.visited.insert(identity))
    }
}

/// An I/O error that says which entry it is about.
fn named(path: &Path, error: std::io::Error) -> std::io::Error {
    std::io::Error::new(error.kind(), format!("{}: {error}", path.display()))
}

/// The `.tt` files of `inputs`, as absolute paths.
pub(crate) fn collect_tt(inputs: &[String]) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for input in inputs {
        collect_sources(Path::new(input), false, &mut files)?;
    }
    files
        .into_iter()
        .filter(|f| crate::SourceKind::from_tt_path(f).is_some())
        .map(|f| super::paths::canonical(&f))
        .collect()
}

/// Whether `path` names a source this compiler takes: a tt source always,
/// and hand-written TypeScript when pass-through is on.
fn is_source(path: &Path, include_ts: bool) -> bool {
    path.extension().is_some_and(|e| {
        TT_EXTENSIONS.iter().any(|tt| *tt == e)
            || (include_ts && TS_EXTENSIONS.iter().any(|ts| *ts == e))
    })
}

/// The extensions [`is_source`] accepts, for an error that has to name them.
fn source_extensions(include_ts: bool) -> String {
    let mut names: Vec<String> = TT_EXTENSIONS.iter().map(|e| format!(".{e}")).collect();
    if include_ts {
        names.extend(TS_EXTENSIONS.iter().map(|e| format!(".{e}")));
    }
    names.join(", ")
}

/// Follow explicit tt edges from the projection's content-version metadata.
/// TypeScript still owns admission; discovery only supplies candidate modules.
fn discover_imports(
    file: &Path,
    imports: &[crate::TtImport],
    overlays: &HashMap<PathBuf, String>,
    pending: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
    imported: &mut HashSet<PathBuf>,
) {
    for import in imports {
        if !(import.specifier.starts_with('.') || Path::new(&import.specifier).is_absolute()) {
            continue;
        }
        let target = file.parent().unwrap_or(Path::new(".")).join(import.path());
        let target = super::paths::canonical(&target).ok().or_else(|| {
            super::normalize_document_path(&target)
                .ok()
                .filter(|path| overlays.contains_key(path))
        });
        let Some(target) = target else {
            continue;
        };
        imported.insert(target.clone());
        if seen.insert(target.clone()) {
            pending.push(target);
        }
    }
}
