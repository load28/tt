//! Typed match analysis — one normalized, typed view of every `match`.
//!
//! The pipeline's other phases each look at a `match` through their own
//! keyhole: sema checks rules over raw patterns, codegen emits shapes,
//! and the engine's language surface asks TypeScript about whatever byte
//! the editor points at. What none of them had was a shared, *typed*
//! description of the match itself — which constructors the scrutinee can
//! be, what each pattern binding's payload type is, what an arm body sees.
//! This module is that description, in the mold of rustc's typed pattern
//! representation (surface pattern → analysis with types attached), sized
//! to tt's contract: ttc does not grow a TypeScript type system, so the
//! types here are the *declared* field types from variant declarations.
//!
//! Layering follows the compiler's existing seams exactly:
//!
//! - This module is a pure pipeline phase like [`crate::probe`] and
//!   [`crate::sema`]: text (plus the same imported-declaration input sema
//!   takes) in, analysis out. No file system, no TypeScript.
//! - The *authoritative* types are still TypeScript's. The engine's
//!   language surface asks the checker first — through the same service
//!   seam every other editor answer travels — and falls back to this
//!   analysis when the checker cannot be asked (an or-pattern binding span
//!   has no emitted counterpart; the toolchain may be absent entirely).
//!   That priority is the module's design contract, not an accident: see
//!   `docs/design/match-analysis.md`.
//! - Exhaustiveness is *computed* here and *reported* by [`crate::sema`]:
//!   the declaration table (local > imported > built-in), the covering
//!   rule, and the tuple product all live in this module, and sema turns
//!   the resulting [`Coverage`] into positioned errors. One rule, one
//!   implementation.
//!
//! The two maps the analysis keeps apart are the point of the model:
//!
//! - **Pattern bindings** ([`PatternBinding`]) — one entry per binding
//!   *occurrence*, keyed by its span. In `A(x) | B(x)` the two `x`
//!   occurrences are two entries with two different payload types.
//! - **Body bindings** ([`BodyBinding`]) — one entry per bound *name*, the
//!   alternatives' types merged (`A`'s payload `| B`'s payload), which is
//!   what the arm body actually sees.
//!
//! Merging these early is exactly the bug this model exists to prevent.

mod coverage;
mod patterns;
mod usefulness;

#[cfg(test)]
mod tests;

use crate::ast::*;
use crate::{ExternVariant, VariantSymbol};

pub(crate) use coverage::checked_coverage;
use coverage::*;
pub(crate) use patterns::has_nested;
use patterns::*;
use usefulness::{Alphabets, Cell, ColTy};

/// The typed analysis of every pattern in one source file, nested ones
/// included, in source order. Produced by [`pattern_analyses`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PatternAnalyses {
    /// Every `match`, in source order (outer before nested).
    pub matches: Vec<MatchAnalysis>,
    /// Every pattern site that is not a match arm — a let-else, an
    /// `if let` — in source order. A match arm's site lives inside its
    /// [`MatchAnalysis`]; these are the constructs that carry exactly one
    /// pattern and no arms.
    pub sites: Vec<PatternSite>,
    /// The names in patterns that did not resolve to a declaration *and*
    /// that the analysis can name a replacement for — the resolution
    /// phase's answer, sorted by position. Reported by [`crate::sema`],
    /// exactly as [`Coverage`] is: one rule, one implementation.
    pub unresolved: Vec<UnresolvedName>,
    /// Every name in a pattern that **did** resolve, with the span it was
    /// written at — the other half of the resolution phase's answer.
    ///
    /// This is what an editor asks for: a case tag and a payload field name
    /// exist nowhere in the emitted TypeScript (a tag becomes a string
    /// literal, a field a destructuring key), so the checker cannot be
    /// asked about them and tt has to answer. Sorted by position.
    pub resolved: Vec<ResolvedName>,
    /// The declaration table the analysis resolved against — local variants
    /// first, then imported ones, then the built-ins, each name once.
    ///
    /// Exposed because the same table answers questions no single pattern
    /// does: what a case's payload looks like (hover), which cases exist
    /// (completion).
    pub declarations: Vec<DeclaredVariant>,
}

/// The complete semantic answer for one parsed file.
///
/// HIR identity, name resolution, and normalized pattern facts are kept
/// together so diagnostics, lowering, and language tooling consume one
/// computation instead of rebuilding adjacent views of the same file.
#[derive(Debug)]
pub(crate) struct SemanticFile {
    pub hir: crate::hir::HirFile,
    pub resolution: crate::resolve::Resolution,
    pub patterns: PatternAnalyses,
}

/// One payload column's alphabet as a type checker named it: which
/// `(constructor, field)` column, and the `kind` literals its type admits.
///
/// This is the one thing the declaration table cannot work out — a field's
/// declared type text may be a type parameter, or name a union no tt
/// declaration describes (`docs/design/rust-parity-analysis.md` §10.3).
pub(crate) type PayloadAlphabet = ((String, String), Vec<String>);

/// One variant of the analysis' declaration table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredVariant {
    /// The variant's name in the analyzed file's scope (an import alias, or
    /// `ns.Name` for a namespace import).
    pub name: String,
    /// Where the declaration came from.
    pub origin: Origin,
    /// The constructors, in declaration order.
    pub constructors: Vec<MatchConstructor>,
}

/// One name in a pattern that resolved to a declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedName {
    /// Whether it was written in tag position or as a field.
    pub kind: NameKind,
    /// The name as written.
    pub name: String,
    /// Byte span of the name as written.
    pub start: usize,
    /// End of the name's span.
    pub end: usize,
    /// The variant it resolved in.
    pub variant_name: String,
    /// Where that variant was declared.
    pub origin: Origin,
    /// For a field, the case whose payload it belongs to.
    pub tag: Option<String>,
}

/// Which construct a pattern was written in — the wording an error uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteKind {
    /// A let-else statement's pattern.
    LetElse,
    /// An `if let` statement's pattern (each link of an `else if let`
    /// chain is its own site).
    IfLet,
}

/// One pattern written outside a `match`: a let-else, an `if let`. The
/// same two questions a match arm answers — what the pattern is over, and
/// what each binding's declared type is — asked of the constructs that
/// carry a single pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternSite {
    /// Which construct this is.
    pub kind: SiteKind,
    /// Byte offset of the construct's keyword (the declaration keyword for
    /// a let-else, `if` for an `if let`) — the site's identity.
    pub keyword_off: usize,
    /// What the pattern is over, when its tag names a variant the analysis
    /// knows. `None` is not an error: a tag pattern also matches any
    /// hand-written tagged union (`language.md` §3.2).
    pub subject: Option<MatchSubject>,
    /// Every binding occurrence in the pattern, nested leaves included —
    /// the same span-keyed map a match arm keeps.
    pub pattern_bindings: Vec<PatternBinding>,
}

/// A name in a pattern that names no declaration, together with the name
/// it was probably meant to be.
///
/// The suggestion is not decoration: ttc does not know the scrutinee's
/// type, so "this tag resolves to nothing" is *not by itself* an error —
/// a tag pattern legitimately matches hand-written tagged unions whose
/// tags no declaration table holds. What licenses the report is that the
/// analysis can also say what to write instead
/// (`docs/design/rust-parity-analysis.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedName {
    /// Whether the name was written in tag position or as a field.
    pub kind: NameKind,
    /// The name as written.
    pub name: String,
    /// Byte span of the name as written.
    pub start: usize,
    /// End of the name's span.
    pub end: usize,
    /// The variant the name was resolved against.
    pub variant_name: String,
    /// Where that variant was declared — an error names the origin.
    pub origin: Origin,
    /// For a field, the case whose payload it was looked up in.
    pub tag: Option<String>,
    /// The declared name it looks like a misspelling of.
    pub suggestion: String,
    /// The `match` keyword this reportable error owns, when it was written in
    /// a match pattern. `None` for `if let` and let-else sites, which have no
    /// exhaustiveness or match-glue consequences to suppress.
    pub match_owner: Option<usize>,
}

/// What kind of name an [`UnresolvedName`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameKind {
    /// A case tag (`Circel` in `Circel(r)`).
    Case,
    /// A payload field name (`radiuz` in `Circle(radiuz)`).
    Field,
}

/// One `match`, normalized: its subject(s), its arms with their typed
/// bindings, and its coverage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchAnalysis {
    /// Byte offset of the `match` keyword — the analysis' identity, shared
    /// with [`crate::TagMatch::offset`] and the emitted scrutinee temporary
    /// ([`crate::ScrutineeTemp::src`]).
    pub keyword_off: usize,
    /// Byte offset just past the match's head — `match (scrutinee)`, arms
    /// excluded. The head is what a diagnostic about the match as a whole
    /// (a hole in its coverage) is drawn over.
    pub head_end: usize,
    /// Byte offset of the body's opening `{`.
    pub body_open: usize,
    /// Byte offset of the body's closing `}`. With [`MatchAnalysis::
    /// body_open`] this is where an arm the compiler authors goes — a
    /// coverage hole's fix is an edit, and the edit needs the braces
    /// (TASK-216).
    pub body_close: usize,
    /// One subject per scrutinee position: one entry for a single match,
    /// one per position for a tuple match. `None` when the position's arm
    /// tags belong to no known variant — the match still analyzes, its
    /// declared types are simply unknown (TypeScript may still know).
    pub subjects: Vec<Option<MatchSubject>>,
    /// The arms, in source order.
    pub arms: Vec<AnalyzedArm>,
    /// The exhaustiveness answer — for a single match over its subject's
    /// tags, for a tuple match over the product of its positions. `None`
    /// when the question does not arise: a wildcard arm covers everything,
    /// the tags identify no known variant, or the match is a literal one
    /// (whose exhaustiveness is a question about a TypeScript type —
    /// [`crate::literal_matches`]). This is what sema reports on; there is
    /// no second implementation of the rule.
    pub coverage: Option<Coverage>,
}

/// What a match is over: the resolved variant and its constructors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchSubject {
    /// The variant's name in this file's scope (alias or `ns.Name` for an
    /// imported one).
    pub variant_name: String,
    /// The variant's constructors, in declaration order.
    pub constructors: Vec<MatchConstructor>,
}

/// One constructor (case) of a subject variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchConstructor {
    /// The case tag.
    pub tag: String,
    /// `None` for a unit case; `Some` for a case with a (possibly empty)
    /// field list.
    pub fields: Option<Vec<PayloadField>>,
}

/// One field of a payload-carrying constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadField {
    /// The field name.
    pub name: String,
    /// Whether the field is optional (`name?: T`) — a destructured binding
    /// then sees `T | undefined`.
    pub optional: bool,
    /// The verbatim declared type text.
    pub ty: String,
}

/// One analyzed arm: where its body is, and the two binding maps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzedArm {
    /// Byte offset of the arm's pattern — where the arm begins, and what a
    /// consumer points at when it has something to say about the arm
    /// itself rather than about its body.
    pub pattern_start: usize,
    /// Byte span of the arm's body (braces excluded for block bodies).
    pub body_start: usize,
    /// End of the body span.
    pub body_end: usize,
    /// Every binding occurrence in the arm's pattern, alternatives kept
    /// apart — the span-keyed map.
    pub pattern_bindings: Vec<PatternBinding>,
    /// Every bound name the body sees, alternatives merged — the name-keyed
    /// map.
    pub body_bindings: Vec<BodyBinding>,
}

/// One binding occurrence inside a pattern: `x` in `A(x)`, `alias` in
/// `A(field: alias)`, a leaf of a nested pattern. In an or-pattern each
/// alternative contributes its own occurrences with its own constructor's
/// payload type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternBinding {
    /// The name the pattern binds (the alias when the binding is aliased).
    pub name: String,
    /// Byte span of the bound name as written.
    pub start: usize,
    /// End of the bound name's span.
    pub end: usize,
    /// The constructor whose payload this occurrence destructures — the
    /// innermost one for a nested pattern's leaf.
    pub tag: String,
    /// The declared type of the destructured field (`| undefined` applied
    /// for an optional field). `None` when the subject — or, for a nested
    /// leaf, the field's variant — is unknown, or the constructor has no such
    /// field.
    pub ty: Option<String>,
    /// The variant [`PatternBinding::ty`] was read from, when it is known.
    pub variant_name: Option<String>,
    /// Byte span of the whole alternative list this occurrence belongs to
    /// (`A(x) | B(x)` for either `x`) — what a consumer replaces when it
    /// isolates one alternative.
    pub group_start: usize,
    /// End of the alternative list's span.
    pub group_end: usize,
    /// Byte span of this occurrence's own top-level alternative (`A(x)` or
    /// `B(x)`) — what the isolated group is replaced *with*.
    pub alt_start: usize,
    /// End of the alternative's span.
    pub alt_end: usize,
    /// How many alternatives the group has. `1` means the emitted
    /// destructuring maps this span already; more means it does not (the
    /// destructuring speaks for every alternative at once), which is when
    /// a consumer needs [`PatternBinding::group_start`]..[`PatternBinding::alt_end`].
    pub alternatives: usize,
}

/// One bound name as an arm's body sees it: the alternatives' payload
/// types merged in source order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyBinding {
    /// The bound name.
    pub name: String,
    /// The merged declared type — a single type when every alternative
    /// agrees, a `A | B` union otherwise. `None` when any alternative's
    /// type is unknown (a partial union would claim more than is known).
    pub ty: Option<String>,
}

/// The exhaustiveness answer for one `match` — the single source sema's
/// error and every other consumer read (`docs/design/match-analysis.md` §5).
///
/// A single match is the arity-1 case: one position, one tag per row. A
/// tuple match enumerates the cartesian product of its positions, so a row
/// is a combination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coverage {
    /// One entry per scrutinee position: the variant whose cases that position
    /// enumerates. `None` for a *universal* position of a tuple match —
    /// every arm writes `_` there, so it constrains nothing.
    pub positions: Vec<Option<CoveredVariant>>,
    /// Tags an arm covers outright — unguarded and nested-free, since a
    /// guard may be false and a nested pattern reaches further in. Single
    /// matches only. This is not what exhaustiveness is computed from (the
    /// algorithm descends into payloads); it is the flat summary a
    /// consumer wants when it asks "which cases are already written here?"
    pub covered: Vec<String>,
    /// **Witnesses**: the values the arms leave unhandled. Empty when the
    /// match is exhaustive; bounded, so a wide product does not build a
    /// list nobody can read.
    pub missing: Vec<Uncovered>,
    /// Indices of arms that match nothing an earlier arm has not already
    /// matched — dead code, in source order.
    ///
    /// Nothing reports these yet: an unreachable arm is a *lint* in Rust,
    /// and tt has only errors, so turning it into one would reject
    /// programs that compile today. It is computed here because the same
    /// recursion answers it, and because the editor is where a hint
    /// belongs (TASK-101 §P3). The narrower duplicate-arm rule sema
    /// enforces is unchanged.
    pub unreachable: Vec<usize>,
}

/// One value a match leaves unhandled, as the tt pattern that would cover
/// it (`Wrap(inner: Yes)`, `_` at a universal position of a tuple match) —
/// one entry per scrutinee position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uncovered {
    /// The pattern text, one entry per scrutinee position.
    pub pattern: Vec<String>,
    /// The same witness written as an **arm pattern**: a field the witness
    /// does not constrain is bound by name rather than dropped, so the
    /// text can be inserted as an arm and its body can use the payload
    /// (`Circle` in a message is `Circle(radius)` in an arm). One entry
    /// per scrutinee position, like [`Uncovered::pattern`].
    pub arm: Vec<String>,
    /// Whether every position was decided from a **known** constructor
    /// set.
    ///
    /// `false` means some column's alphabet was not identifiable — a
    /// hand-written union, a payload whose declared type names no variant —
    /// so this witness is tt's best guess rather than its knowledge. The
    /// default compile path reports it anyway (a conservative "you may
    /// have missed something" is the only answer available without types);
    /// the typed path does not, because there the honest move is to ask
    /// the checker (`docs/design/rust-parity-analysis.md` §10.3).
    pub certain: bool,
}

impl Coverage {
    /// The arity-1 view of [`Coverage::missing`] — the patterns a single
    /// match leaves uncovered. Empty for a tuple match's coverage.
    pub fn missing_tags(&self) -> Vec<&str> {
        if self.positions.len() != 1 {
            return Vec::new();
        }
        self.missing
            .iter()
            .filter_map(|row| row.pattern.first().map(String::as_str))
            .collect()
    }
}

/// A variant a [`Coverage`] position enumerates, with where it was declared —
/// the origin an error message names ("variant E", "built-in variant Option",
/// "variant T (imported from \"./token.tt\")").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoveredVariant {
    /// The variant's name in this file's scope.
    pub name: String,
    /// Where the declaration came from.
    pub origin: Origin,
}

/// Where a declaration in the analysis' table came from. Resolution runs
/// local > imported > built-in, so a nearer origin shadows a farther one of
/// the same name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// Declared in the file being analyzed.
    Local,
    /// Imported from another module.
    Imported {
        /// The specifier as written (`./token.tt`), when the collector
        /// recorded it — an error message quotes it to say *which* variant.
        from: Option<String>,
    },
    /// A built-in variant (`Option`, `Result`).
    Builtin,
}

impl PatternAnalyses {
    /// Whether a reportable resolver error owns this match.
    ///
    /// The resolver-error list is the diagnostic source and the suppression
    /// source. There is no copied boolean that can silence a checker result
    /// after its corresponding cause was dropped.
    pub(crate) fn match_has_resolution_error(&self, keyword_off: usize) -> bool {
        self.unresolved
            .iter()
            .any(|error| error.match_owner == Some(keyword_off))
    }

    /// The pattern binding whose bound-name span contains `offset` (end
    /// inclusive, matching how the language surface treats a chunk's end).
    ///
    /// ```
    /// let src = "variant E { A(x: string), B(x: number) }\nconst v = match (e) { A(x) | B(x) => x };\n";
    /// let analyses = ttc::pattern_analyses(src, &[]);
    /// let a_x = analyses.binding_at(src.find("A(x)").unwrap() + 2).unwrap();
    /// assert_eq!(a_x.ty.as_deref(), Some("string"));
    /// let b_x = analyses.binding_at(src.find("B(x)").unwrap() + 2).unwrap();
    /// assert_eq!(b_x.ty.as_deref(), Some("number"));
    /// ```
    pub fn binding_at(&self, offset: usize) -> Option<&PatternBinding> {
        self.matches
            .iter()
            .flat_map(|m| &m.arms)
            .flat_map(|a| &a.pattern_bindings)
            .chain(self.sites.iter().flat_map(|s| &s.pattern_bindings))
            .find(|b| b.start <= offset && offset <= b.end)
    }

    /// Where the name under `offset` — a reference inside an arm's body —
    /// was bound: the pattern-binding spans of that name in the innermost
    /// enclosing arm. Empty when `offset` is not on such a reference.
    ///
    /// This is a *fallback* answer by design: it does not model shadowing
    /// inside the body, so a consumer asks it only when the checker had no
    /// answer of its own (the or-pattern destructuring is compiler glue,
    /// which navigation never lands in).
    pub fn body_definitions(&self, source: &str, offset: usize) -> Vec<(usize, usize)> {
        let Some((start, end)) = identifier_at(source, offset) else {
            return Vec::new();
        };
        let name = &source[start..end];
        for arm in self.enclosing_arms(offset) {
            let spans: Vec<(usize, usize)> = arm
                .pattern_bindings
                .iter()
                .filter(|b| b.name == name)
                .map(|b| (b.start, b.end))
                .collect();
            if !spans.is_empty() {
                return spans;
            }
        }
        Vec::new()
    }

    /// The body binding the name under `offset` refers to, with the
    /// identifier's span — the name-keyed lookup, same fallback contract as
    /// [`PatternAnalyses::body_definitions`].
    pub fn body_binding_at(
        &self,
        source: &str,
        offset: usize,
    ) -> Option<(&BodyBinding, (usize, usize))> {
        let (start, end) = identifier_at(source, offset)?;
        let name = &source[start..end];
        for arm in self.enclosing_arms(offset) {
            if let Some(binding) = arm.body_bindings.iter().find(|b| b.name == name) {
                return Some((binding, (start, end)));
            }
        }
        None
    }

    /// The arms whose body contains `offset`, innermost first: a nested
    /// match's arm body sits inside the outer arm's, and the nearer binding
    /// is the one a name resolves to.
    fn enclosing_arms(&self, offset: usize) -> Vec<&AnalyzedArm> {
        let mut arms: Vec<&AnalyzedArm> = self
            .matches
            .iter()
            .flat_map(|m| &m.arms)
            .filter(|a| a.body_start <= offset && offset < a.body_end)
            .collect();
        arms.sort_by_key(|a| a.body_end - a.body_start);
        arms
    }
}

/// Analyzes every `match` of a source file, nested ones included, in
/// source order. `externs` are imported variant declarations under their
/// in-scope names — the same input [`crate::Options::extern_variants`] gives
/// sema, carried as [`VariantSymbol`]s because the analysis needs field
/// types, not just tags. Subjects resolve local > imported > built-in
/// (`Option`, `Result`), exactly as exhaustiveness does.
///
/// ```
/// let src = "variant E { A(x: string), B(x: number) }\nconst v = match (e) { A(x) => x, B(x) => x };\n";
/// let analyses = ttc::pattern_analyses(src, &[]);
/// let subject = analyses.matches[0].subjects[0].as_ref().unwrap();
/// assert_eq!(subject.variant_name, "E");
/// assert_eq!(analyses.matches[0].arms[0].body_bindings[0].ty.as_deref(), Some("string"));
/// ```
///
/// let-else and `if let` are analyzed the same way, as [`PatternSite`]s:
///
/// ```
/// let src = "variant E { A(x: string) }\nif let A(x) = e { use(x); }\n";
/// let analyses = ttc::pattern_analyses(src, &[]);
/// assert_eq!(analyses.sites[0].pattern_bindings[0].ty.as_deref(), Some("string"));
/// ```
pub fn pattern_analyses(source: &str, externs: &[VariantSymbol]) -> PatternAnalyses {
    let program = crate::parser::parse(source);
    let decls: Vec<crate::resolve::ExternDecl> = externs.iter().map(Into::into).collect();
    semantics_over(source, &program, &decls, Depth::Full).patterns
}

/// The coverage-only analysis of an already-parsed program — sema's input,
/// so exhaustiveness and the editor's model answer from one implementation
/// of the rule.
///
/// `externs` are the imported declarations the CLI collects for sema
/// ([`crate::Options::extern_variants`]). The resolver still validates their
/// case and field names; only binding-type analysis is skipped, so every arm
/// comes back empty — [`pattern_analyses`] is the entry point for bindings.
#[cfg(test)]
pub(crate) fn coverage_analyses(
    source: &str,
    program: &Program,
    externs: &[ExternVariant],
) -> PatternAnalyses {
    coverage_semantics(source, program, externs).patterns
}

/// Builds the semantic file used by sema and lowering in the ordinary
/// compiler pipeline.
pub(crate) fn coverage_semantics(
    source: &str,
    program: &Program,
    externs: &[ExternVariant],
) -> SemanticFile {
    let decls: Vec<crate::resolve::ExternDecl> = externs.iter().map(Into::into).collect();
    semantics_over(source, program, &decls, Depth::CoverageOnly)
}

/// The one pipeline both entry points run: lower, resolve, build the
/// declaration table **from the resolver's world** (one construction of
/// the rules — local later-wins, imports shadowed by locals, built-ins by
/// both), analyze, then attach the resolver's name answers.
fn semantics_over(
    source: &str,
    program: &Program,
    externs: &[crate::resolve::ExternDecl],
    depth: Depth,
) -> SemanticFile {
    let mut hir = crate::hir::lower_program(crate::hir::FileId(0), source, program);
    let resolution = crate::resolve::resolve_file(&mut hir, externs);
    let table = Table::from_resolution(&resolution);
    let mut analyses = analyze(program, &table, depth);
    attach_resolution(&mut analyses, &hir, &resolution);
    let semantics = SemanticFile {
        hir,
        resolution,
        patterns: analyses,
    };
    validate_semantic(&semantics);
    semantics
}

/// Checks cross-phase identity links. Failure means a compiler bug, never a
/// user error: parser recovery must already be explicit in HIR/resolution.
fn validate_semantic(file: &SemanticFile) {
    for node in file.resolution.uses.keys() {
        assert!(
            file.hir.source_map.node_span(*node).is_some(),
            "resolved HIR use has no source span"
        );
    }
    for unresolved in &file.resolution.unresolved {
        assert!(
            file.hir.source_map.node_span(unresolved.node).is_some(),
            "unresolved HIR use has no source span"
        );
    }
    for analysis in &file.patterns.matches {
        assert!(
            file.hir.sites.iter().any(|(_, site)| {
                file.hir
                    .source_map
                    .node_span(site.node)
                    .is_some_and(|span| span.start == analysis.keyword_off)
            }),
            "match analysis has no HIR pattern site"
        );
    }
}

/// Copies the resolver's answers into the analysis' vocabulary:
/// [`PatternAnalyses::unresolved`] and [`PatternAnalyses::resolved`]. Name
/// resolution has **one** implementation — [`crate::resolve`] — and this is
/// where its answers enter the surface sema and the editor already consume.
fn attach_resolution(
    analyses: &mut PatternAnalyses,
    hir: &crate::hir::HirFile,
    resolution: &crate::resolve::Resolution,
) {
    use crate::resolve as res;

    let span_of = |node: crate::hir::NodeId| {
        hir.source_map
            .node_span(node)
            .map_or((0, 0), |s| (s.start, s.end))
    };
    let variant_origin = |def: crate::hir::DefId| match &resolution.defs[def].kind {
        res::DefKind::Variant(data) => match &data.origin {
            res::DeclOrigin::Local(_) => Origin::Local,
            res::DeclOrigin::Imported { from } => Origin::Imported { from: from.clone() },
            res::DeclOrigin::Builtin => Origin::Builtin,
        },
        res::DefKind::VariantValue { .. } => Origin::Local,
    };

    for miss in &resolution.unresolved {
        let (start, end) = span_of(miss.node);
        let site = &hir.sites[miss.site];
        let match_owner = matches!(
            site.kind,
            crate::hir::SiteKind::Match | crate::hir::SiteKind::TupleMatch
        )
        .then(|| span_of(site.node).0);
        analyses.unresolved.push(UnresolvedName {
            kind: match miss.kind {
                res::UseKind::Case => NameKind::Case,
                res::UseKind::Field => NameKind::Field,
            },
            name: miss.name.clone(),
            start,
            end,
            variant_name: resolution.defs[miss.against].name.clone(),
            origin: variant_origin(miss.against),
            tag: miss.tag.clone(),
            suggestion: miss.suggestion.clone(),
            match_owner,
        });
    }
    for (&node, answer) in &resolution.uses {
        let (start, end) = span_of(node);
        match answer {
            res::Res::Variant(v) => analyses.resolved.push(ResolvedName {
                kind: NameKind::Case,
                name: resolution
                    .variant(*v)
                    .map(|d| d.name.clone())
                    .unwrap_or_default(),
                start,
                end,
                variant_name: resolution.defs[v.variant_def].name.clone(),
                origin: variant_origin(v.variant_def),
                tag: None,
            }),
            res::Res::Field(f) => analyses.resolved.push(ResolvedName {
                kind: NameKind::Field,
                name: resolution
                    .field(*f)
                    .map(|d| d.name.clone())
                    .unwrap_or_default(),
                start,
                end,
                variant_name: resolution.defs[f.variant.variant_def].name.clone(),
                origin: variant_origin(f.variant.variant_def),
                tag: resolution.variant(f.variant).map(|d| d.name.clone()),
            }),
            _ => {}
        }
    }
    // Source order, so the error a file reports does not depend on which
    // construct the resolver happened to reach first.
    analyses.unresolved.sort_by_key(|u| u.start);
    analyses.resolved.sort_by_key(|r| r.start);
}
