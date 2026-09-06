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

use super::projection::{self, ProjectedDocument};
use super::semantics::{self, Checked, FileSemantics};
use super::snapshot::Snapshot;
use crate::CompileError;
use crate::typescript::backend::{FailureKind, TypeScriptBackend};
use crate::typescript::native::NativeBackend;

/// What counts as a tt source, and what counts as hand-written TypeScript.
const TT_EXTENSIONS: &[&str] = &["tt", "ttx"];
const TS_EXTENSIONS: &[&str] = &["ts", "tsx", "mts", "cts"];

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
    /// Report only the tt layer. The type layer is TypeScript's answer about
    /// the user's own code, and a caller that already has it from somewhere
    /// else (an editor with a live language server) would show it twice.
    pub tt_only: bool,
}

/// One workspace's compiler state: documents, projections, and the session.
#[derive(Debug)]
pub struct Project {
    pub(crate) root: PathBuf,
    tsconfig: Option<PathBuf>,
    /// The output tree a scan must not descend into (`--types`'s sidecar
    /// directory).
    out_dir: Option<PathBuf>,
    /// The inputs' `.tt` files — what a `--types` run writes. The TypeScript
    /// program owns graph membership; this only narrows emission.
    requested: HashSet<PathBuf>,
    /// Candidate files for the first layered-filesystem pass, fixed at open:
    /// the project scan, or the inputs when the scan found nothing. The
    /// configured TypeScript program filters these to actual members.
    initial: Vec<PathBuf>,
    /// The project's hand-written TypeScript, listed only when there is no
    /// `tsconfig.json` to decide the program's files — see
    /// [`crate::typescript::backend::Query::sources`].
    sources: Vec<PathBuf>,
    /// Unsaved text standing in for files on disk, keyed by canonical path.
    pub(crate) overlays: HashMap<PathBuf, String>,
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
            out_dir,
            requested: collected.into_iter().collect(),
            initial,
            sources,
            overlays: HashMap::new(),
            cache: HashMap::new(),
            backend,
            pattern_analysis_cache: RefCell::new(HashMap::new()),
            pattern_analysis_cache_hits: Cell::new(0),
            next_snapshot: 0,
            service: None,
        }
    }

    /// The project root — the directory the compiler runs in.
    pub fn root(&self) -> &Path {
        &self.root
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
    pub fn open_document(&mut self, path: PathBuf, text: String) {
        self.overlays.insert(path, text);
    }

    /// Replaces an open document's text. The next [`Project::update`] sees
    /// the new text; snapshots already taken keep the old one.
    pub fn update_document(&mut self, path: PathBuf, text: String) {
        self.overlays.insert(path, text);
    }

    /// Closes an open document: the file's text is the disk's again.
    pub fn close_document(&mut self, path: &Path) {
        self.overlays.remove(path);
    }

    /// Every candidate `.tt` file under the project root, as sorted absolute
    /// paths. TypeScript later decides which candidates are configured or
    /// reachable. Scanned fresh so a newly created file is seen.
    pub fn scan(&self) -> std::io::Result<Vec<PathBuf>> {
        project_sources(&self.root, self.out_dir.as_deref(), TT_EXTENSIONS)
    }

    /// The candidate set the first pass layers, decided when the project was
    /// opened: the project scan, or — when that found nothing (inputs outside
    /// the root) — the inputs themselves.
    pub fn initial_files(&self) -> Vec<PathBuf> {
        self.initial.clone()
    }

    /// Takes a snapshot of `files` as they are now: overlay text where a
    /// document is open, disk text otherwise. A file whose text is unchanged
    /// since the last snapshot keeps its projection; the rest are
    /// re-projected. A file that cannot lower remains in the snapshot as a
    /// blocked source with its tt diagnostics; other files still project.
    /// An I/O failure still blocks the snapshot because no source state is
    /// available to preserve.
    pub fn update(&mut self, files: &[PathBuf]) -> Result<Snapshot, Box<Blocked>> {
        let mut projected = Vec::with_capacity(files.len());
        let mut blocked_files = Vec::new();
        let mut cache = HashMap::with_capacity(files.len());
        for file in files {
            let text = match self.overlays.get(file) {
                Some(text) => text.clone(),
                None => std::fs::read_to_string(file).map_err(|e| {
                    Box::new(Blocked {
                        path: file.clone(),
                        error: CompileError {
                            message: format!("cannot read: {e}"),
                            filename: Some(file.display().to_string()),
                            line: 0,
                            col: 0,
                            end_line: 0,
                            end_col: 0,
                        },
                    })
                })?,
            };
            let doc = match self.cache.get(file) {
                Some(cached) if cached.source == text => Some(cached.clone()),
                _ => match ProjectedDocument::project_for_snapshot(file, text) {
                    Ok(doc) => Some(Arc::new(doc)),
                    Err(blocked) => {
                        blocked_files.push(Arc::new(blocked));
                        None
                    }
                },
            };
            if let Some(doc) = doc {
                cache.insert(file.clone(), doc.clone());
                projected.push(doc);
            }
        }
        // Entries for files that left the project go with the old map; a
        // blocked update above leaves the previous cache intact instead, so
        // the files that were fine keep their projections.
        self.cache = cache;
        self.next_snapshot += 1;
        Ok(Snapshot {
            id: self.next_snapshot,
            files: projected,
            blocked: blocked_files,
            host_overlays: self
                .overlays
                .iter()
                .filter(|(path, _)| is_host_source(path))
                .map(|(path, text)| (path.clone(), text.clone()))
                .collect(),
        })
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
        externs: Vec<crate::VariantSymbol>,
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
        let analyses = crate::pattern_analyses(source, &externs);
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
                let text = match self.overlays.get(target) {
                    Some(text) => text.clone(),
                    None => std::fs::read_to_string(target).ok()?,
                };
                if let Some(doc) = self.cache.get(target)
                    && doc.source == text
                {
                    return Some(
                        doc.variant_symbols()
                            .iter()
                            .filter(|d| d.exported)
                            .cloned()
                            .collect(),
                    );
                }
                Some(
                    crate::variant_symbols_with_kind(
                        &text,
                        crate::SourceKind::from_path(target).unwrap_or_default(),
                    )
                    .into_iter()
                    .filter(|d| d.exported)
                    .collect(),
                )
            },
        );
        self.pattern_analysis(path, source, externs)
    }

    /// Checks a snapshot: asks the running compiler about it and returns
    /// diagnostics at `.tt` positions — and the emitted declarations, when
    /// the request wants them. The session persists across calls; only what
    /// changed since the last ask travels.
    pub fn check(&self, snapshot: &Snapshot, request: &CheckRequest) -> Result<Checked, String> {
        let semantics = self.file_semantics(snapshot);
        let (mut query, probes) = projection::assemble(
            snapshot.files(),
            snapshot.blocked(),
            &self.root,
            &self.sources,
        );
        query.emit_declarations = request.emit_declarations;
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
        let declarations = if request.emit_declarations && backend_error.is_none() {
            semantics::match_declarations(snapshot, &answers, &self.root, &self.requested)
        } else {
            Default::default()
        };
        Ok(Checked {
            diagnostics: semantics::report(
                snapshot,
                &answers,
                &probes,
                request.tt_only,
                &semantics,
                &self.requested,
            ),
            declarations,
            backend_error,
        })
    }
}

/// Host files retain their original paths and syntax in backend overlays.
pub(super) fn is_host_source(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "ts" || extension == "tsx")
}

/// One cached [`FileSemantics`] with the half of its key the value does
/// not carry (the content hash; the externs are compared on the value).
#[derive(Debug)]
struct CachedPatternAnalysis {
    source_hash: u64,
    value: Arc<FileSemantics>,
}

/// Every file of the project with one of `extensions`, as absolute paths.
/// `node_modules`, dot directories and the output tree are skipped — nothing
/// there is a source.
pub(crate) fn project_sources(
    root: &Path,
    out_dir: Option<&Path>,
    extensions: &[&str],
) -> std::io::Result<Vec<PathBuf>> {
    let out = out_dir.and_then(|d| d.canonicalize().ok());
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if Some(&dir) == out.as_ref() {
            continue;
        }
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.starts_with('.') || name == "node_modules" {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| extensions.contains(&e))
            {
                files.push(path.canonicalize()?);
            }
        }
    }
    files.sort();
    Ok(files)
}

/// The nearest `tsconfig.json` at or above the inputs' common directory.
pub(crate) fn find_tsconfig(files: &[PathBuf]) -> Option<PathBuf> {
    let mut dir = files.first()?.parent()?.to_path_buf();
    loop {
        let candidate = dir.join("tsconfig.json");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Collects sources under `entry` the way every ttc mode does: a file is
/// taken as it is; a directory is walked recursively, skipping
/// dot-directories and `node_modules`, taking `.tt` — and, when
/// `include_ts` is set, hand-written TypeScript (`.ts`/`.mts`/`.cts`) too.
pub fn collect_sources(
    entry: &Path,
    include_ts: bool,
    out: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    let meta = std::fs::metadata(entry)?;
    if meta.is_file() {
        // A named file is filtered the same way the walk filters one: the
        // contract is about extensions, not about how the file was reached.
        // Without this, `ttc -o build src/app.js` wrote TypeScript syntax
        // into a file still called `.js`.
        if !is_source(entry, include_ts) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "not a tt or TypeScript source (expected {})",
                    source_extensions(include_ts)
                ),
            ));
        }
        out.push(entry.to_path_buf());
        return Ok(());
    }
    if meta.is_dir() {
        let mut children: Vec<PathBuf> = std::fs::read_dir(entry)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        children.sort();
        for child in children {
            let meta = std::fs::metadata(&child)?;
            if meta.is_dir() {
                // Dot-directories (.git, .tt-build, .tt-types, ...) and
                // node_modules are never sources; descending into them
                // would pull generated or vendored TypeScript into the
                // build — or the cache tree into itself.
                let skip = child.file_name().is_some_and(|name| {
                    let name = name.to_string_lossy();
                    name.starts_with('.') || name == "node_modules"
                });
                if !skip {
                    collect_sources(&child, include_ts, out)?;
                }
            } else if meta.is_file() && is_source(&child, include_ts) {
                out.push(child);
            }
        }
    }
    Ok(())
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
        .map(|f| f.canonicalize())
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
