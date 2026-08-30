//! The seam: what tt asks TypeScript, in tt's terms.
//!
//! Nothing here mentions a process, a protocol, or a compiler version — that
//! is [`super::native`]'s business. tt features build a [`Query`] and read an
//! [`Answers`]; swapping how the compiler is reached changes neither.
//!
//! A query is a **batch**. One round trip carries every question about a
//! project, because the transport is a real IPC channel and asking a hundred
//! questions one at a time costs a hundred round trips.
//!
//! In the compiler core's vocabulary (`docs/design/compiler-core.md` §7)
//! [`Query`] **is** the type-request set and [`Answers`] **is** the typed
//! facts: the snapshot's questions are collected first (the engine's
//! `projection::assemble`), sent as one batch, and the answers are tt-owned
//! — constructor domains as tag lists ([`TagMembers`]), symbol identity as
//! a snapshot-wide id ([`Resolution`]), mutation verdicts as the `builtin`
//! flag the `val` report interprets. There is deliberately no per-expression
//! oracle: a chatty ask-per-node interface would put a round trip inside
//! every analysis loop. A backend that cannot run degrades every typed fact
//! to unknown ([`Answers::default`]) — it never takes the tt layer down
//! with it (`engine::Project::check`).
//!
//! The port is intentionally semantic-only. SWC owns TypeScript parsing,
//! whole-program AST structure, and generated-output syntax verification
//! inside the compiler process. Whether a distribution requires a compatible
//! TypeScript 7 installation is an availability contract for this port, not a
//! reason to expand [`Query`] into a general parsing or lowering interface.

use std::path::PathBuf;

/// Why the TypeScript boundary could not answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FailureKind {
    /// The required process or compatible toolchain is not available.
    Unavailable,
    /// The compiler boundary ran and broke its own protocol or execution
    /// contract. This is a compiler failure, never a source diagnostic.
    Internal,
}

/// A failure of the TypeScript boundary, classified before it reaches a
/// consumer so a missing installation is never presented as a compiler bug
/// and a backend crash is never presented as a missing installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Failure {
    pub kind: FailureKind,
    pub message: String,
}

impl Failure {
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Unavailable,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            kind: FailureKind::Internal,
            message: message.into(),
        }
    }
}

/// One module of the project as TypeScript should see it: the ordinary
/// TypeScript an `.tt` file lowers to, at the path that `.tt` file occupies
/// with a `.ts` extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Module {
    /// Absolute path, `.ts` extension — the name TypeScript resolves.
    pub path: PathBuf,
    /// The emitted TypeScript.
    pub text: String,
}

/// "Which of these literals does the scrutinee's type still allow?" — the
/// typed half of literal-`match` exhaustiveness.
///
/// The position names the scrutinee **as it appears in the emitted module**,
/// so the answer reflects the type TypeScript computes at that point,
/// narrowing included. ttc never reconstructs the declared type.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LiteralQuery {
    /// The module the scrutinee lives in.
    pub module: PathBuf,
    /// UTF-16 offset of the scrutinee in that module.
    pub position: usize,
    /// The literals the match's unguarded arms cover.
    pub covered: Vec<crate::Literal>,
}

/// "Which case tags does the scrutinee's type still allow?" — the same
/// question as [`LiteralQuery`], for a tt variant. The variant lowers to a
/// discriminated union, so the answer is the `kind` literals of the type's
/// constituents at that point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TagQuery {
    /// The module the scrutinee lives in.
    pub module: PathBuf,
    /// UTF-16 offset of the scrutinee in that module.
    pub position: usize,
    /// The tags the match's unguarded arms cover.
    pub covered: Vec<String>,
}

/// "What is the symbol here?" — the primitive `val` is built from.
///
/// Two identifiers name the same binding when they resolve to the same
/// symbol, and a method is a built-in when its symbol is declared in
/// TypeScript's own lib files. Both are questions about resolution, so both
/// are asked as one, and tt does the interpreting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SymbolQuery {
    /// The module the identifier lives in.
    pub module: PathBuf,
    /// UTF-16 offset of the identifier in that module.
    pub position: usize,
}

/// Whether the type at a position is definitely the two-case Result shape.
///
/// This is deliberately structural: aliases and generic instantiations are
/// accepted when the checker proves both literal `kind` cases and their
/// payload fields; unions, `any`, and type parameters produce no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResultShapeQuery {
    pub module: PathBuf,
    pub position: usize,
}

/// Everything asked of one project graph, in one round trip.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Query {
    /// The lowered `.tt` modules. Their text is served from memory —
    /// nothing is written to disk.
    pub modules: Vec<Module>,
    /// Hand-written `.ts` files of the project, by path: the compiler reads
    /// them from disk, where they already are. Listing them matters only
    /// when the workspace has no `tsconfig.json` — then the project is
    /// inferred from what is opened, and a `.ts` file nothing imports would
    /// otherwise never be checked. With a `tsconfig.json` the project's own
    /// `include` decides and this stays empty.
    pub sources: Vec<PathBuf>,
    pub literals: Vec<LiteralQuery>,
    pub tags: Vec<TagQuery>,
    pub symbols: Vec<SymbolQuery>,
    pub result_shapes: Vec<ResultShapeQuery>,
    /// Ask the compiler to emit the lowered modules' `.d.ts` as well. ttc
    /// never writes declaration syntax of its own: the compiler emits for a
    /// lowered module exactly what it would for a hand-written one.
    pub emit_declarations: bool,
}

/// One TypeScript diagnostic, in TypeScript's coordinates. Mapping it back
/// to a position in the `.tt` source is [`super::mapper`]'s job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Diagnostic {
    /// The file TypeScript reported it in.
    pub file: PathBuf,
    /// UTF-16 offsets in that file.
    pub start: usize,
    pub end: usize,
    /// TypeScript's error code (`2322` for `TS2322`).
    pub code: u32,
    pub message: String,
    /// Structured assignability facts when the checker identified the
    /// expression and its contextual type. The raw message remains the
    /// lossless fallback; renderers prefer these facts.
    pub mismatch: Option<TypeMismatch>,
    /// The checker's own related places — "the expected type comes from
    /// this declaration", "first declared here" — each in the coordinates
    /// of the file it names. Empty when the checker offered none.
    pub related: Vec<RelatedInformation>,
}

/// One place the checker relates a diagnostic to, in that file's own
/// UTF-16 coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RelatedInformation {
    pub file: PathBuf,
    pub start: usize,
    pub end: usize,
    pub message: String,
}

/// A checker-proven type mismatch, independent of the syntax that placed
/// the expression in a typed context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TypeMismatch {
    /// UTF-16 range of the expression whose type was compared.
    pub start: usize,
    pub end: usize,
    /// The complete contextual and actual types.
    pub expected: String,
    pub found: String,
    /// Minimal incompatible leaves found by descending through unions and
    /// matching generic aliases. Empty means no safe reduction was found.
    pub differences: Vec<TypeDifference>,
    /// The declaration of the symbol used as the mismatched expression,
    /// when the checker resolved one. This lets the reporter connect a
    /// later diagnostic to the lowering that introduced the value without
    /// guessing from identifier text or source proximity.
    pub declaration: Option<RelatedInformation>,
}

/// The smallest expected/found pair the checker can prove incompatible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TypeDifference {
    pub expected: String,
    pub found: String,
}

/// The literals a [`LiteralQuery`]'s arms fail to cover. Present only when
/// the checker found a **definite** finite union of literals; an indefinite
/// type produces no answer at all rather than a guess.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LiteralMissing {
    /// Index into [`Query::literals`].
    pub index: usize,
    pub missing: Vec<crate::Literal>,
}

/// The case tags a [`TagQuery`]'s arms fail to cover, under the same
/// certainty rule as [`LiteralMissing`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TagMissing {
    /// Index into [`Query::tags`].
    pub index: usize,
    pub missing: Vec<String>,
}

/// Every case tag a [`TagQuery`]'s scrutinee type can be, under the same
/// certainty rule as [`LiteralMissing`] — the *members*, not what the arms
/// left out.
///
/// This is the checker as an **oracle for one column**: tt runs its own
/// exhaustiveness algorithm over the answer, which is how a hole inside a
/// payload gets seen at all (`docs/design/rust-parity-analysis.md` §10.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TagMembers {
    /// Index into [`Query::tags`].
    pub index: usize,
    /// The `kind` literals the scrutinee's type still allows, in the
    /// order the type lists them.
    pub tags: Vec<String>,
}

/// What a [`SymbolQuery`] resolved to. A position that resolved to nothing
/// — an `any` receiver, an unresolved name — has no entry at all, and an
/// unresolved question never becomes a tt error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Resolution {
    /// Index into [`Query::symbols`].
    pub index: usize,
    /// The symbol's id. Snapshot-wide, so two identifiers naming the same
    /// binding share it — across modules included.
    pub id: i64,
    /// The symbol's name.
    pub name: String,
    /// Whether every declaration of this symbol is one of TypeScript's own
    /// — the compiler's answer, which is what makes a method a built-in
    /// rather than a user-defined one that shares a name.
    pub builtin: bool,
}

/// One emitted declaration file, in memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Declaration {
    /// The path the compiler would have written it to.
    pub path: PathBuf,
    pub text: String,
}

/// The answers to one [`Query`].
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Answers {
    /// Lowered modules that TypeScript admitted to the configured program.
    /// `None` means the backend did not run; an empty program is different
    /// from an unavailable answer.
    pub project_modules: Option<Vec<PathBuf>>,
    pub diagnostics: Vec<Diagnostic>,
    pub literal_missing: Vec<LiteralMissing>,
    pub tag_missing: Vec<TagMissing>,
    pub tag_members: Vec<TagMembers>,
    pub resolutions: Vec<Resolution>,
    pub result_shapes: Vec<ResultShape>,
    pub declarations: Vec<Declaration>,
}

/// A checker-proven Result shape answer. Absent answers remain unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResultShape {
    pub index: usize,
}

/// A source of TypeScript semantics for one project.
///
/// The project is named by its `tsconfig.json`: the compiler owns
/// configuration, module resolution and the file list, and ttc only adds the
/// modules it lowered.
pub(crate) trait TypeScriptBackend {
    /// Answers every question of `query` against the project `tsconfig`
    /// configures, or — when there is none — against a project inferred for
    /// the query's own modules, rooted at `root`. Returns a human-readable
    /// message when the backend itself could not run, never when the *code*
    /// has errors, which are [`Answers::diagnostics`].
    fn ask(
        &self,
        tsconfig: Option<&std::path::Path>,
        root: &std::path::Path,
        query: &Query,
    ) -> Result<Answers, Failure>;
}
