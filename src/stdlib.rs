//! The tt standard library as tree-shakeable TypeScript modules.
//!
//! The standard library is three TypeScript modules, and pipeline lowering
//! uses one compiler-owned runtime module. The CLI materializes only the
//! modules a project needs and bundler adapters expose them virtually;
//! user-written imports otherwise pass through the compiler untouched,
//! so the passthrough contract is unaffected. The values inside are
//! byte-identical to what the corresponding tt `enum`s would compile to
//! (guarded by `tests/stdlib.rs`), which is what makes `match` — and the
//! built-in exhaustiveness check below — work on them. `Result`'s two
//! constructors are the one deliberate deviation: they are typed by the
//! variant each builds (`Ok<T>` / `Err<E>`) rather than by the whole
//! `TResult<T, E>`, so a function with several `try`s infers a union of the
//! real error types instead of `unknown`.

/// TypeScript source of the `@tt/std` type-only entry point.
pub const STD_TYPES_SOURCE: &str = include_str!("stdlib/types.ts");

/// TypeScript source of the `@tt/std/option` runtime module.
pub const STD_OPTION_SOURCE: &str = include_str!("stdlib/option.ts");

/// TypeScript source of the `@tt/std/result` runtime module.
pub const STD_RESULT_SOURCE: &str = include_str!("stdlib/result.ts");

/// TypeScript source of the compiler-owned pipeline runtime module.
pub const RUNTIME_SOURCE: &str = include_str!("stdlib/runtime.ts");

/// The bare specifier a `.tt` file uses for standard-library types.
///
/// It is bare rather than relative on purpose: a relative path would have
/// to name a file that only exists after generation, and TypeScript's
/// `paths` — the mapping an editor needs — does not apply to relative
/// specifiers. The `ttc` CLI writes the module into the output tree and
/// rewrites this specifier to point at it; a bundler plugin can serve it
/// as a virtual module instead.
pub const STD_SPECIFIER: &str = "@tt/std";

/// One physical module of the standard-library package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StdModule {
    /// `@tt/std`, the type-only entry point.
    Types,
    /// `@tt/std/option`, the Option constructors and combinators.
    Option,
    /// `@tt/std/result`, the Result constructors and combinators.
    Result,
    /// `@tt/runtime`, the compiler-owned pipeline helpers.
    Runtime,
}

impl StdModule {
    /// User-facing standard-library modules, excluding compiler runtime.
    pub const STANDARD: [StdModule; 3] = [StdModule::Types, StdModule::Option, StdModule::Result];
    /// All modules in deterministic materialization order.
    pub const ALL: [StdModule; 4] = [
        StdModule::Types,
        StdModule::Option,
        StdModule::Result,
        StdModule::Runtime,
    ];

    /// The bare module specifier users write.
    pub const fn specifier(self) -> &'static str {
        match self {
            StdModule::Types => STD_SPECIFIER,
            StdModule::Option => "@tt/std/option",
            StdModule::Result => "@tt/std/result",
            StdModule::Runtime => "@tt/runtime",
        }
    }

    /// The module's file name inside the generated `tt/` directory.
    pub const fn file_name(self) -> &'static str {
        match self {
            StdModule::Types => "index.ts",
            StdModule::Option => "option.ts",
            StdModule::Result => "result.ts",
            StdModule::Runtime => "runtime.ts",
        }
    }

    /// The module's embedded TypeScript source.
    pub const fn source(self) -> &'static str {
        match self {
            StdModule::Types => STD_TYPES_SOURCE,
            StdModule::Option => STD_OPTION_SOURCE,
            StdModule::Result => STD_RESULT_SOURCE,
            StdModule::Runtime => RUNTIME_SOURCE,
        }
    }

    pub(crate) fn from_specifier(specifier: &[u8]) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|module| specifier == module.specifier().as_bytes())
    }
}

/// Per-module compiler support rewrites supplied by a build adapter.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdImports<'a> {
    /// Replacement for `@tt/std`.
    pub types: Option<&'a str>,
    /// Replacement for `@tt/std/option`.
    pub option: Option<&'a str>,
    /// Replacement for `@tt/std/result`.
    pub result: Option<&'a str>,
    /// Replacement for the compiler-generated `@tt/runtime` import.
    pub runtime: Option<&'a str>,
}

impl<'a> StdImports<'a> {
    pub(crate) const fn get(self, module: StdModule) -> Option<&'a str> {
        match module {
            StdModule::Types => self.types,
            StdModule::Option => self.option,
            StdModule::Result => self.result,
            StdModule::Runtime => self.runtime,
        }
    }
}

// The built-in enums a file gets without declaring them (`Option`,
// `Result`) live in [`crate::analysis`], with their payload fields: one
// declaration table serves both exhaustiveness and the editor's types, so
// there is no tag-only copy of them here.
