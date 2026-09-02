//! tt — a tiny preprocessor language that compiles to TypeScript and TSX.
//!
//! Every valid TypeScript file is a valid `.tt` file, and every valid TSX file
//! is a valid `.ttx` file. Both compile to themselves byte for byte; the
//! compiler only rewrites the constructs tt adds —
//! Rust-style `variant` declarations (TypeScript enums pass through
//! untouched), `match` expressions (literal, tuple and nested patterns
//! included), `try` statements (Rust-`?`-style error propagation over
//! `Result`), let-else and `if let` statements, and the pipeline operator
//! `|>` — plus the `val` binding modifier, which is erased, and relative
//! `.tt`/`.ttx` import specifiers, which are rewritten to a consumable form (see
//! [`ImportRewrite`]). tt-level errors — duplicate cases, non-exhaustive
//! matches, bad field types, misplaced `try`, mutation through a `val`
//! binding — are ttc compile errors with exact positions; the emitted
//! output is plain TypeScript.
//!
//! The core public API is [`compile`] plus its [`Options`] (with
//! [`ImportRewrite`]) and error type [`CompileError`] — code, or the first
//! error. The multi-diagnostic forms are [`analyze`] (every tt-level
//! [`Diagnostic`], in source order) and [`compile_report`] (the same, plus
//! the emission when one is possible); the tree-shakeable standard library
//! modules are exposed through [`StdModule`] and the `STD_*_SOURCE`
//! constants. The `ttc` binary in this crate is a thin CLI over it.
//!
//! # Example
//!
//! ```
//! use ttc::{compile, Options};
//!
//! let source = r#"
//! export variant Shape {
//!   Circle(radius: number),
//!   Point,
//! }
//!
//! const area = match (Shape.Circle(2)) {
//!   Circle(radius) => Math.PI * radius * radius,
//!   Point => 0,
//! };
//! "#;
//!
//! let ts = compile(source, &Options::default())?;
//! assert!(ts.contains(r#"{ kind: "Circle"; radius: number }"#));
//! assert!(ts.contains("switch ($tt_m.kind)"));
//! # Ok::<(), ttc::CompileError>(())
//! ```
//!
//! # Documentation
//!
//! - `README.md` / `README.ko.md` — installation, language overview, and
//!   contributor setup.
//! - `ttc help <topic>` — the language and workflow guide embedded in the CLI.
//! - `docs/design/` — architecture and design decisions.

mod analysis;
#[path = "lib/api.rs"]
mod api;
mod ast;
mod codegen;
#[path = "lib/compile.rs"]
mod compile;
mod core_ir;
mod diagnostics;
pub mod engine;
mod error;
mod evaluation_ir;
pub mod flow;
pub mod hir;
pub mod ice;
mod lexer;
#[path = "lib/mapped.rs"]
mod mapped;
mod parser;
mod probe;
mod program_syntax;
pub mod render;
pub mod resolve;
mod scanner;
mod sema;
mod sidecar;
pub mod source_map;
mod stdlib;
pub(crate) mod typescript;
mod val;
mod verify;

pub use analysis::{
    AnalyzedArm, BodyBinding, Coverage, CoveredVariant, MatchAnalysis, MatchConstructor,
    MatchSubject, NameKind, Origin, PatternAnalyses, PatternBinding, PatternSite, PayloadField,
    SiteKind, UnresolvedName, pattern_analyses,
};
pub use diagnostics::{Diagnostic, DiagnosticCode, DiagnosticOwner, Edit, Severity, Suggestion};
pub use error::CompileError;
pub use probe::{
    Literal, LiteralMatch, PayloadProbe, TagMatch, literal_matches, literal_matches_with_kind,
    payload_probes, payload_probes_with_kind, tag_matches, tag_matches_with_kind,
};
pub use sidecar::{Sidecar, build_sidecar};
pub use stdlib::{
    RUNTIME_SOURCE, STD_OPTION_SOURCE, STD_RESULT_SOURCE, STD_SPECIFIER, STD_TYPES_SOURCE,
    StdImports, StdModule,
};
pub use val::{Mutation, ValBinding, ValFn, ValParam, ValPass, ValProbes, is_builtin_mutator_name};

pub use api::*;
pub use compile::*;
pub use mapped::*;

#[cfg(test)]
#[path = "lib/mapped_result_tests.rs"]
mod mapped_result_tests;

use error::TtError;
