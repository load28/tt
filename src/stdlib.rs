//! The tt standard library as tree-shakeable TypeScript modules.
//!
//! The standard library is three TypeScript modules, and pipeline lowering
//! uses one compiler-owned runtime module. The CLI materializes only the
//! modules a project needs and bundler adapters expose them virtually;
//! user-written imports otherwise pass through the compiler untouched,
//! so the passthrough contract is unaffected. The values inside are
//! byte-identical to what the corresponding tt `variant`s would compile to
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

const STD_TYPES_DECLARATION: &str = include_str!("stdlib/commonjs/types.d.ts");
const STD_OPTION_DECLARATION: &str = include_str!("stdlib/commonjs/option.d.ts");
const STD_RESULT_DECLARATION: &str = include_str!("stdlib/commonjs/result.d.ts");
const RUNTIME_DECLARATION: &str = include_str!("stdlib/commonjs/runtime.d.ts");

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

    /// TypeScript's declaration emit for [`StdModule::source`], which the
    /// package's CommonJS entry points serve.
    pub const fn declaration(self) -> &'static str {
        match self {
            StdModule::Types => STD_TYPES_DECLARATION,
            StdModule::Option => STD_OPTION_DECLARATION,
            StdModule::Result => STD_RESULT_DECLARATION,
            StdModule::Runtime => RUNTIME_DECLARATION,
        }
    }

    pub(crate) fn from_specifier(specifier: &[u8]) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|module| specifier == module.specifier().as_bytes())
    }
}

/// One installable package the standard-library modules are served as.
///
/// The package is dual-format so that both module formats resolve it in
/// every `moduleResolution` mode. The package root is `"type": "module"`
/// and holds the ES-module sources. `cjs/` holds their declarations under a
/// `"type": "commonjs"` manifest. A declaration file is ambient, so
/// `verbatimModuleSyntax` accepts its `export` modifiers in a CommonJS
/// module, where a source file's are TS1287. Each `"exports"` entry maps the
/// `import` condition to the root file and the `require` condition's `types`
/// to the `cjs/` declaration, with `types` first in each. Both conditions'
/// `default` is the root file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StdPackage {
    /// `@tt/std`: the type-only entry point and the Option/Result modules.
    Std,
    /// `@tt/runtime`: the compiler-owned pipeline helpers.
    Runtime,
}

/// The subdirectory holding the CommonJS copy of a [`StdPackage`].
pub const STD_PACKAGE_COMMONJS_DIR: &str = "cjs";

const COMMONJS_MANIFEST: &str = "{\n  \"type\": \"commonjs\"\n}\n";

impl StdPackage {
    /// Every package, in deterministic materialization order.
    pub const ALL: [StdPackage; 2] = [StdPackage::Std, StdPackage::Runtime];

    /// The package name, which is also its `node_modules` directory.
    pub const fn name(self) -> &'static str {
        match self {
            StdPackage::Std => STD_SPECIFIER,
            StdPackage::Runtime => "@tt/runtime",
        }
    }

    /// The modules the package contains.
    pub const fn modules(self) -> &'static [StdModule] {
        match self {
            StdPackage::Std => &StdModule::STANDARD,
            StdPackage::Runtime => &[StdModule::Runtime],
        }
    }

    /// The file a module occupies inside its package directory.
    pub const fn file_name(module: StdModule) -> &'static str {
        match module {
            StdModule::Runtime => "index.ts",
            _ => module.file_name(),
        }
    }

    /// The file a module's declaration occupies inside the package's
    /// [`STD_PACKAGE_COMMONJS_DIR`].
    pub fn commonjs_file_name(module: StdModule) -> String {
        let file = Self::file_name(module);
        format!("{}.d.ts", file.strip_suffix(".ts").unwrap_or(file))
    }

    /// The package's `package.json`.
    pub fn manifest(self) -> String {
        self.manifest_with(|module| {
            (
                format!(
                    "./{STD_PACKAGE_COMMONJS_DIR}/{}",
                    Self::commonjs_file_name(module)
                ),
                format!("./{}", Self::file_name(module)),
            )
        })
    }

    fn manifest_with(self, require: impl Fn(StdModule) -> (String, String)) -> String {
        let entries = self
            .modules()
            .iter()
            .map(|module| {
                let subpath = &module.specifier()[self.name().len()..];
                let file = Self::file_name(*module);
                let (types, default) = require(*module);
                format!(
                    "    \".{subpath}\": {{\n      \"import\": {{ \"types\": \"./{file}\", \"default\": \"./{file}\" }},\n      \"require\": {{ \"types\": \"{types}\", \"default\": \"{default}\" }}\n    }}"
                )
            })
            .collect::<Vec<_>>()
            .join(",\n");
        format!(
            "{{\n  \"name\": \"{name}\",\n  \"version\": \"0.0.0\",\n  \"type\": \"module\",\n  \"types\": \"./index.ts\",\n  \"exports\": {{\n{entries}\n  }}\n}}\n",
            name = self.name()
        )
    }

    /// The files earlier ttc releases wrote beside the root modules, one list
    /// per layout. A package directory holding exactly one of them was
    /// written by ttc and is upgraded in place by [`StdPackage::materialize`].
    /// The first layout has no `"type"` and no `"exports"`. The second
    /// served `cjs/` copies of the sources.
    fn earlier_layouts(self) -> [Vec<(String, String)>; 2] {
        let legacy = vec![(
            "package.json".to_string(),
            format!(
                "{{\n  \"name\": \"{}\",\n  \"version\": \"0.0.0\",\n  \"types\": \"index.ts\"\n}}\n",
                self.name()
            ),
        )];
        let copied = self
            .modules()
            .iter()
            .map(|module| {
                (
                    format!("{STD_PACKAGE_COMMONJS_DIR}/{}", Self::file_name(*module)),
                    format!("{GENERATED_BANNER}{}", module.source()),
                )
            })
            .chain([
                (
                    format!("{STD_PACKAGE_COMMONJS_DIR}/package.json"),
                    COMMONJS_MANIFEST.to_string(),
                ),
                (
                    "package.json".to_string(),
                    self.manifest_with(|module| {
                        let copy =
                            format!("./{STD_PACKAGE_COMMONJS_DIR}/{}", Self::file_name(module));
                        (copy.clone(), copy)
                    }),
                ),
            ])
            .collect();
        [legacy, copied]
    }

    /// The CommonJS entry points of the package as `(path, text)` pairs
    /// relative to the package directory.
    fn commonjs_files(self, banner: &str) -> Vec<(String, String)> {
        self.modules()
            .iter()
            .map(|module| {
                (
                    format!(
                        "{STD_PACKAGE_COMMONJS_DIR}/{}",
                        Self::commonjs_file_name(*module)
                    ),
                    format!("{banner}{}", module.declaration()),
                )
            })
            .chain(std::iter::once((
                format!("{STD_PACKAGE_COMMONJS_DIR}/package.json"),
                COMMONJS_MANIFEST.to_string(),
            )))
            .collect()
    }

    /// Every file of the package as `(path, text)` pairs relative to the
    /// package directory, each module prefixed with `banner`.
    pub fn files_with_banner(self, banner: &str) -> Vec<(String, String)> {
        self.modules()
            .iter()
            .map(|module| {
                (
                    Self::file_name(*module).to_string(),
                    format!("{banner}{}", module.source()),
                )
            })
            .chain(self.commonjs_files(banner))
            .chain(std::iter::once((
                "package.json".to_string(),
                self.manifest(),
            )))
            .collect()
    }

    /// Every file of the package, each module with [`GENERATED_BANNER`].
    pub fn files(self) -> Vec<(String, String)> {
        self.files_with_banner(GENERATED_BANNER)
    }

    /// The package's directory under `root`.
    pub fn directory(self, root: &std::path::Path) -> std::path::PathBuf {
        root.join("node_modules").join(self.name())
    }

    /// Writes the package into `root/node_modules` when it is absent. A
    /// package that exists is left alone, except that one holding exactly
    /// the files of one of [`StdPackage::earlier_layouts`] gets the current
    /// manifest and CommonJS entry points in their place.
    pub fn materialize(self, root: &std::path::Path) -> std::io::Result<()> {
        let directory = self.directory(root);
        let files = if !directory.exists() {
            self.files()
        } else if let Some(earlier) = self.earlier_layouts().into_iter().find(|layout| {
            layout.iter().all(|(name, text)| {
                std::fs::read_to_string(directory.join(name)).is_ok_and(|found| found == *text)
            })
        }) {
            let mut files = self.commonjs_files(GENERATED_BANNER);
            files.push(("package.json".to_string(), self.manifest()));
            for (name, _) in earlier {
                if !files.iter().any(|(current, _)| *current == name) {
                    std::fs::remove_file(directory.join(name))?;
                }
            }
            files
        } else {
            return Ok(());
        };
        for (name, text) in files {
            let path = directory.join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, text)?;
        }
        Ok(())
    }
}

/// The first line of every standard-library file ttc writes for a project.
pub const GENERATED_BANNER: &str = "// @generated by ttc --emit-std — do not edit directly.\n";

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

// The built-in variants a file gets without declaring them (`Option`,
// `Result`) live in [`crate::analysis`], with their payload fields: one
// declaration table serves both exhaustiveness and the editor's types, so
// there is no tag-only copy of them here.
