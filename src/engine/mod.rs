//! The tt language engine — one authoritative project state for every
//! consumer.
//!
//! ttc's batch build compiles files one at a time and forgets them; that
//! mode needs no engine (the same way `tsc` runs without tsserver). This
//! module is for everything else: type checking, watching, editors, and
//! external tooling — the consumers that need a **live** project whose
//! answers stay consistent while files change.
//!
//! The shape follows typescript-go's project service, sized to tt:
//!
//! - an [`Engine`] discovers the toolchain and opens projects;
//! - a [`Project`] is the long-lived, mutable state of one workspace —
//!   documents (disk and unsaved overlays), cached projections, and the
//!   running TypeScript session;
//! - a [`Snapshot`] is the project at one moment, immutable; every semantic
//!   request runs against a snapshot, so a request started before an edit
//!   still answers about a consistent state;
//! - the projection ([`ProjectedDocument`]) — tt source, generated
//!   TypeScript, byte-exact mappings, probes — is the engine's own layer,
//!   the one thing tt has that a plain TypeScript engine does not.
//!
//! Results come back in tt's own vocabulary ([`Diagnostic`], [`Checked`]):
//! TypeScript's unstable API surface stays behind
//! [`crate::typescript::backend`] and never reaches a consumer.
//!
//! ```no_run
//! use ttc::engine::{CheckRequest, Engine, ProjectOptions};
//!
//! let engine = Engine::new(None);
//! let mut project = engine
//!     .open_project(&["src".to_string()], &ProjectOptions::default())
//!     .unwrap();
//! let files = project.initial_files();
//! let snapshot = project.update(&files).unwrap();
//! let checked = project.check(&snapshot, &CheckRequest::default()).unwrap();
//! for d in &checked.diagnostics {
//!     eprintln!("{}: {}", d.path.display(), d.message);
//! }
//! ```

mod completions;
mod declarations;
mod hints;
mod language;
mod names;
mod project;
mod projection;
mod semantics;
mod snapshot;
mod tokens;

pub use completions::{TtCompletion, TtCompletionKind, tt_completions_at};
pub use declarations::{
    TtCaseDecl, TtDeclarations, TtFieldDecl, TtMatchSite, TtVariantDecl, TtVariantOrigin,
    tt_declarations,
};
pub use hints::{TtHint, TtHintKind, tt_hints};
pub use language::{
    CompletionAnswer, CompletionDetail, CompletionItem, HoverInfo, Location, Position,
    RENAME_PLACEHOLDER, Range, Reference, RenameEdit, ServiceDiagnostic, ServiceRelated, Signature,
    SignatureHelp, SignatureParameter,
};
pub use names::{TtSymbol, TtSymbolKind, tt_symbol_at};
pub use project::{Blocked, CheckRequest, Project, collect_sources};
pub use projection::ProjectedDocument;
pub use semantics::{
    BackendError, BackendErrorKind, Checked, Declarations, Diagnostic, DiagnosticLabel,
    ModuleDeclaration,
};
pub use snapshot::Snapshot;
pub use tokens::{SemanticToken, SemanticTokenKind, semantic_tokens, semantic_tokens_with_kind};

use std::path::PathBuf;

use crate::typescript::native::NativeBackend;

/// Process-wide entry point: toolchain discovery and project creation.
#[derive(Debug, Default)]
pub struct Engine {
    /// The `node` binary that runs the TypeScript host, or `None` for the
    /// `node` on PATH.
    node: Option<PathBuf>,
}

/// How a project is opened, beside its inputs.
#[derive(Debug, Default)]
pub struct ProjectOptions {
    /// The `tsconfig.json` to check against, when named (`--project`).
    /// Otherwise the nearest one at or above the inputs decides, and a
    /// workspace without one still works — the compiler infers a project.
    pub tsconfig: Option<PathBuf>,
    /// A directory of generated output (the sidecar tree) that scans must
    /// not descend into.
    pub out_dir: Option<PathBuf>,
}

impl Engine {
    /// An engine that runs its TypeScript host with `node` (or the PATH's
    /// `node` when `None`). Nothing is started until a project's first
    /// question.
    pub fn new(node: Option<PathBuf>) -> Engine {
        Engine { node }
    }

    /// Opens the project `inputs` belong to.
    ///
    /// The project's own configuration is what the checker runs with: the
    /// named `tsconfig` or the nearest one above the inputs. Candidate `.tt`
    /// files under the root are layered so tsconfig globs and imports can
    /// discover them; TypeScript's configured program then decides which
    /// candidates join. What was named decides only what an emitting pass
    /// writes.
    ///
    /// The error is a ready-to-print sentence: nothing collected, an
    /// unreadable input, or no TypeScript toolchain.
    pub fn open_project(
        &self,
        inputs: &[String],
        options: &ProjectOptions,
    ) -> Result<Project, String> {
        let collected = match project::collect_tt(inputs) {
            Ok(files) if files.is_empty() => return Err("no .tt or .ttx sources found".to_string()),
            Ok(files) => files,
            Err(e) => return Err(e.to_string()),
        };
        let (tsconfig, root) = identity_of(&collected, options);
        self.open_collected(collected, tsconfig, root, options)
    }

    /// Resolve an editor buffer's project without filtering out host sources.
    pub fn document_project_identity(
        path: &std::path::Path,
        options: &ProjectOptions,
    ) -> Result<(Option<PathBuf>, PathBuf), String> {
        let canonical = normalize_document_path(path)?;
        Ok(identity_of(&[canonical], options))
    }

    /// Open a project from any editor buffer, including a host TypeScript file.
    /// Host documents are overlays, not tt lowering inputs.
    pub fn open_document_project(
        &self,
        path: &std::path::Path,
        options: &ProjectOptions,
    ) -> Result<Project, String> {
        let document = normalize_document_path(path)?;
        let (tsconfig, root) = Self::document_project_identity(&document, options)?;
        // Candidate discovery belongs to `open_collected`; `collected` is
        // only what the caller explicitly requested. Keeping those sets
        // distinct prevents an editor question from reporting tt errors in
        // unrelated files that the project's tsconfig excludes.
        let requested = crate::SourceKind::from_tt_path(&document)
            .is_some()
            .then_some(document)
            .into_iter()
            .collect();
        self.open_collected(requested, tsconfig, root, options)
    }

    fn open_collected(
        &self,
        collected: Vec<PathBuf>,
        tsconfig: Option<PathBuf>,
        root: PathBuf,
        options: &ProjectOptions,
    ) -> Result<Project, String> {
        // Scan candidates for the layered filesystem. Membership is not
        // inferred from this walk: the configured TypeScript program admits
        // include/files roots and everything reachable through its graph.
        //
        // The inputs join the scan rather than standing in for it when it
        // comes back empty. A file the caller named is a root by request —
        // the same rule the typed report already applies to them — so a
        // named file the project's own `include` leaves out is still
        // snapshotted, and still answers for its tt layer.
        let mut initial =
            match project::project_sources(&root, options.out_dir.as_deref(), &["tt", "ttx"]) {
                Ok(all) => all,
                Err(e) => return Err(e.to_string()),
            };
        initial.extend(collected.iter().cloned());
        initial.sort();
        initial.dedup();
        // No toolchain is not "no project": the tt layer answers without
        // one, and the missing backend is carried as the typed layer's
        // failure instead ([`Checked::backend_error`]).
        let backend = NativeBackend::new(self.node.clone(), &root);
        // With a configuration the project's own `include` decides which
        // hand-written files are in the program; without one, they have to
        // be listed or a `.ts` nothing imports is never checked.
        let sources = match tsconfig {
            Some(_) => Vec::new(),
            None => {
                project::project_sources(&root, options.out_dir.as_deref(), project::TS_EXTENSIONS)
                    .unwrap_or_default()
            }
        };
        Ok(Project::new(
            root,
            tsconfig,
            options.out_dir.clone(),
            collected,
            initial,
            sources,
            backend,
        ))
    }

    /// The identity `inputs` resolve to — the `(tsconfig, root)` pair a
    /// project is opened as. Two input sets with the same identity describe
    /// the same project, which is what a server keys its live sessions by.
    pub fn project_identity(
        inputs: &[String],
        options: &ProjectOptions,
    ) -> Result<(Option<PathBuf>, PathBuf), String> {
        let collected = match project::collect_tt(inputs) {
            Ok(files) if files.is_empty() => return Err("no .tt or .ttx sources found".to_string()),
            Ok(files) => files,
            Err(e) => return Err(e.to_string()),
        };
        Ok(identity_of(&collected, options))
    }
}

/// Gives an editor document a stable absolute identity whether or not its
/// leaf has reached disk yet.
///
/// Existing files keep their fully canonical identity. For a new file, the
/// existing parent directory is canonicalized and the unsaved leaf is joined
/// back onto it. The overlay can then participate in the same project as its
/// saved neighbours without a temporary disk write.
pub fn normalize_document_path(path: &std::path::Path) -> Result<PathBuf, String> {
    if let Ok(canonical) = path.canonicalize() {
        return Ok(canonical);
    }
    let name = path
        .file_name()
        .ok_or_else(|| format!("{} has no document name", path.display()))?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let parent = parent
        .canonicalize()
        .map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(parent.join(name))
}

/// The `(tsconfig, root)` the collected inputs belong to.
fn identity_of(collected: &[PathBuf], options: &ProjectOptions) -> (Option<PathBuf>, PathBuf) {
    let tsconfig = options
        .tsconfig
        .clone()
        .or_else(|| project::find_tsconfig(collected))
        .map(|path| path.canonicalize().unwrap_or(path));
    let root = match &tsconfig {
        Some(path) => path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf(),
        // No configuration: the sources' own directories are the project.
        None => collected
            .first()
            .and_then(|f| f.parent())
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf(),
    };
    (tsconfig, root)
}
