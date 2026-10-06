//! The projection — tt's own layer between a source and the type checker.
//!
//! An `.tt` file enters the TypeScript project as ordinary TypeScript. The
//! [`ProjectedDocument`] is that fact made first-class: one file's source
//! text, the module it becomes, the byte-exact mappings between the two, and
//! every question the file wants asked of the checker (match probes, `val`
//! probes) — computed **once per content version** and reused across
//! snapshots. This is where the engine's incrementality lives: a file whose
//! text did not change between two snapshots costs nothing to re-project.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::snapshot::BlockedFile;
#[cfg(test)]
use crate::CompileError;
use crate::typescript::backend::{
    LiteralQuery, Module, Query, ResultShapeQuery, SymbolQuery, TagQuery,
};
use crate::typescript::mapper;
use crate::{LiteralMatch, MappedEmit, Options, TagMatch, ValProbes};

/// One `.tt` file as every consumer of the engine sees it: the source the
/// user wrote, and the TypeScript the compiler is given — plus everything
/// the engine derives from the text, cached with it.
#[derive(Debug, Clone)]
pub struct ProjectedDocument {
    /// The `.tt` file.
    pub source_path: PathBuf,
    /// The source text — the coordinate space diagnostics are reported in.
    pub source: String,
    /// The path the lowered module occupies in the project: the same place,
    /// with a `.ts` extension (`src/token.tt` → `src/token.tt.ts`).
    pub(crate) module_path: PathBuf,
    /// The emitted TypeScript and its verbatim-chunk mappings.
    pub(crate) emit: MappedEmit,
    /// Whether the file imports any `@tt/std` entry — decides whether the
    /// standard-library package joins the project graph.
    pub(crate) imports_std: bool,
    /// Whether the lowered module imports the compiler-owned pipeline
    /// runtime — what codegen emitted, not whether the source has a pipeline.
    pub(crate) imports_runtime: bool,
    /// The literal-match exhaustiveness probes of this file.
    pub(crate) literal_probes: Vec<LiteralMatch>,
    /// The tag-match exhaustiveness probes of this file.
    pub(crate) tag_probes: Vec<TagMatch>,
    /// The nested patterns of this file — the payload positions the
    /// checker is asked to name the alphabet of.
    pub(crate) payload_probes: Vec<crate::PayloadProbe>,
    /// The `val` bindings, mutations, declarations and passes of this file,
    /// unpaired — pairing is symbol identity, the checker's answer.
    pub(crate) val: ValProbes,
    /// The file's own tt-level diagnostics, found while projecting — the
    /// recoverable ones (a duplicate arm, an unknown case) that do not stop
    /// the file from lowering. The typed pass reports them **alongside**
    /// its own answers, so one tt error no longer hides a file's type
    /// errors and exhaustiveness holes (TASK-117 symptom 3).
    pub(crate) tt_diagnostics: Vec<crate::Diagnostic>,
    /// Source ranges replaced by parser-owned error placeholders in the
    /// typed projection. Diagnostics originating inside these ranges are
    /// recovery effects; diagnostics elsewhere remain reportable.
    pub(crate) recovered: Vec<(usize, usize)>,
    pub(crate) syntax_repairs: Vec<(usize, usize)>,
    /// Whether a recovery stands for one of the file's declarations
    /// ([`crate::ProjectionReport::recovered_declarations`]): the
    /// declarations emitted from this projection are not the file's own.
    pub(crate) recovered_declaration: bool,
    /// Whether `emit` is the faithful projection of a document whose
    /// TypeScript does not parse ([`crate::ProjectionReport::withheld`]).
    /// Only a document held open is checked through one, as an editor
    /// checks a `.ts` buffer mid-edit; no declarations are written from it.
    pub(crate) unparsed: bool,
    /// The variants the file exports, under their exported names, parsed
    /// once per content version — what an importer's extern collection
    /// reads, so a file that did not change is never re-parsed for its
    /// exports (`docs/design/compiler-core.md` §11).
    exported_variant_symbols: std::sync::OnceLock<Vec<crate::VariantSymbol>>,
    /// Relative `.tt` imports collected while projecting this content version.
    /// Shared by snapshot graph discovery and semantic cache dependencies.
    imports: Vec<crate::TtImport>,
}

impl ProjectedDocument {
    /// The TypeScript this document lowered to: the text the checker and
    /// the language service read in its place.
    pub fn code(&self) -> &str {
        &self.emit.code
    }

    /// The variants the file exports ([`crate::exported_variant_symbols`]),
    /// computed on first use and pinned to this projection's content version.
    pub(crate) fn exported_variant_symbols(&self) -> &[crate::VariantSymbol] {
        self.exported_variant_symbols.get_or_init(|| {
            crate::exported_variant_symbols_with_kind(
                &self.source,
                crate::SourceKind::from_path(&self.source_path).unwrap_or_default(),
            )
        })
    }

    /// The dependency edges of this projected content version.
    pub(crate) fn tt_imports(&self) -> &[crate::TtImport] {
        &self.imports
    }
}

impl ProjectedDocument {
    /// Projects one file: lowers it to ordinary TypeScript and derives every
    /// probe the typed pass will ask about. Recoverable tt-level errors ride
    /// along in [`ProjectedDocument::tt_diagnostics`]; only an error that
    /// leaves the file impossible to lower
    /// ([`crate::DiagnosticCode::blocks_projection`]) fails the projection.
    #[cfg(test)]
    pub(crate) fn project(
        source_path: &Path,
        source: String,
    ) -> Result<ProjectedDocument, CompileError> {
        Self::project_for_snapshot(source_path, source, false).map_err(|blocked| {
            blocked
                .diagnostics
                .first()
                .expect("a blocked projection has a diagnostic")
                .to_compile_error(
                    &blocked.source,
                    Some(blocked.source_path.to_string_lossy().as_ref()),
                )
        })
    }

    /// The projection a snapshot holds for `source`. A file whose
    /// TypeScript does not parse is blocked, as `tsc` reports only its
    /// syntax errors while there are any, unless it is `open`: a document
    /// being edited is checked through its faithful projection, so its type
    /// errors keep the checker's facts while a syntax error is transient.
    pub(crate) fn project_for_snapshot(
        source_path: &Path,
        source: String,
        open: bool,
    ) -> Result<ProjectedDocument, BlockedFile> {
        let options = Options {
            filename: Some(source_path.to_str().unwrap_or("<input>")),
            source_kind: crate::SourceKind::from_path(source_path).unwrap_or_default(),
            // Exhaustiveness and `val`'s pairing are the checker's answers
            // here — see `Options::defer_to_checker`.
            defer_to_checker: true,
            // Specifiers stay exactly as written. `"./token.tt"` already
            // names the lowered module ([`module_path_of`]), and `"@tt/std"`
            // entries already name the standard-library package — so the
            // declarations the compiler emits are usable as they are, by a
            // consumer that never sees this compile.
            rewrite_imports: crate::ImportRewrite::Off,
            ..Options::default()
        };
        let source_kind = options.source_kind;
        let (program, tokens) = crate::parser::lex_and_parse_with_kind(&source, source_kind);
        let report = crate::compile_projection_report_parsed(&source, &options, &program, &tokens);
        let (emit, unparsed) = match (report.emit, report.withheld) {
            (Some(emit), _) if !report.editor_only || open => (emit, report.editor_only),
            (None, Some(withheld)) if open => (withheld, true),
            _ => {
                return Err(BlockedFile::new(
                    source_path.to_path_buf(),
                    source,
                    report.diagnostics,
                ));
            }
        };
        let scan = crate::scan_module_of(&source, &program);
        Ok(ProjectedDocument {
            module_path: module_path_of(source_path),
            imports_std: scan.imports_std,
            imports_runtime: emit.support_imports.contains(&crate::StdModule::Runtime),
            literal_probes: crate::probe::literal_matches_of(&source, &program),
            tag_probes: crate::probe::tag_matches_of(&source, &program),
            payload_probes: crate::probe::payload_probes_of(&program),
            val: crate::val_probes_with_emit(&source, source_kind, &program, &tokens, &emit),
            source_path: source_path.to_path_buf(),
            source,
            emit,
            tt_diagnostics: report.diagnostics,
            recovered: report.recovered,
            syntax_repairs: report.syntax_repairs,
            recovered_declaration: !report.recovered_declarations.is_empty(),
            unparsed,
            exported_variant_symbols: std::sync::OnceLock::new(),
            imports: scan.imports,
        })
    }
}

/// The module path an `.tt` file takes in the project graph: its own path
/// with `.ts` appended, so `src/token.tt` becomes `src/token.tt.ts`.
///
/// This is the engine's name for the module. The TypeScript backend decides
/// how the compiler sees it (`src/typescript/host.mjs`). A configured project
/// holds it as `token.tt` through a content mapper, so `"./token.tt"`
/// resolves as it does under `tsc --runExternalCode`. Otherwise it is served
/// as `token.tt.ts`, which ordinary resolution finds where TypeScript probes
/// extensions. Either way the declaration the compiler emits for it lands on
/// `token.tt.d.ts`, the editor sidecar the same specifier resolves to when no
/// compiler is running.
pub(crate) fn module_path_of(source_path: &Path) -> PathBuf {
    let mut name = source_path.as_os_str().to_os_string();
    let kind = crate::SourceKind::from_path(source_path).unwrap_or_default();
    name.push(format!(".{}", kind.output_extension()));
    PathBuf::from(name)
}

/// The standard-library packages a snapshot's files need, in
/// [`crate::StdPackage::ALL`] order.
///
/// Each is a package of the project like any other — served from the same
/// layered file system under `node_modules`, resolved by ordinary node
/// resolution — so the specifier stays bare in the source and in every
/// declaration emitted from it. Nothing is written to the user's
/// `node_modules`.
pub(crate) fn served_std_packages(files: &[Arc<ProjectedDocument>]) -> Vec<crate::StdPackage> {
    crate::StdPackage::ALL
        .into_iter()
        .filter(|package| {
            files.iter().any(|file| match package {
                crate::StdPackage::Std => file.imports_std,
                crate::StdPackage::Runtime => file.imports_runtime,
            })
        })
        .collect()
}

fn std_package_dir(package: crate::StdPackage) -> PathBuf {
    Path::new("node_modules").join(package.name())
}

pub(crate) fn std_package_modules(
    root: &Path,
    package: crate::StdPackage,
) -> impl Iterator<Item = Module> {
    let directory = root.join(std_package_dir(package));
    package
        .files_with_banner("")
        .into_iter()
        .map(move |(name, text)| Module {
            path: directory.join(name),
            text,
        })
}

/// The path the compiler emits a lowered module's declarations to:
/// `src/token.tt.ts` → `src/token.tt.d.ts`, which is the sidecar name a
/// specifier written `"./token.tt"` resolves to.
pub(crate) fn declaration_path_of(file: &ProjectedDocument) -> PathBuf {
    file.module_path.with_extension("d.ts")
}

/// Builds the batch of questions the whole snapshot asks in one round trip.
///
/// Every question is anchored at a byte the compiler can see: a probe whose
/// anchor did not survive lowering as verbatim text (a nested tt construct)
/// is dropped rather than asked about at an approximate position.
pub(crate) fn assemble(
    files: &[Arc<ProjectedDocument>],
    blocked: &[Arc<BlockedFile>],
    root: &Path,
    sources: &[PathBuf],
) -> (Query, Probes) {
    let mut query = Query {
        sources: sources.to_vec(),
        ..Query::default()
    };
    let mut probes = Probes::default();

    for package in served_std_packages(files) {
        query.modules.extend(std_package_modules(root, package));
    }

    for file in files {
        query.modules.push(Module {
            path: file.module_path.clone(),
            text: file.emit.code.clone(),
        });
        if file.unparsed {
            query.unparsed_documents.push(file.module_path.clone());
        }

        for result_return in &file.emit.result_return_temps {
            query.result_shapes.push(ResultShapeQuery {
                module: file.module_path.clone(),
                // Codegen brackets the emitted expression. The backend asks
                // for the smallest expression node covering this exact range,
                // so member calls and non-ASCII identifiers are classified as
                // values rather than by an arbitrary token position.
                start: mapper::to_utf16(&file.emit.code, result_return.out),
                end: mapper::to_utf16(&file.emit.code, result_return.out_end),
            });
            probes.result_returns.push(SourceAnchor {
                source_path: file.source_path.clone(),
                offset: result_return.src,
                end: result_return.src_end,
            });
        }

        for probe in &file.literal_probes {
            if file
                .recovered
                .iter()
                .any(|&(start, end)| start <= probe.offset && probe.offset < end)
            {
                continue;
            }
            let Some(position) = scrutinee_position(&file.emit, probe.offset) else {
                continue;
            };
            query.literals.push(LiteralQuery {
                module: file.module_path.clone(),
                position,
                covered: probe.covered.clone(),
            });
            probes.literals.push(MatchAnchor {
                anchor: SourceAnchor {
                    source_path: file.source_path.clone(),
                    offset: probe.offset,
                    end: scrutinee_end(&file.source, probe.scrutinee_end),
                },
                body_open: probe.body_open,
                body_close: probe.body_close,
                tail: probe.tail,
            });
        }

        for probe in &file.tag_probes {
            if file
                .recovered
                .iter()
                .any(|&(start, end)| start <= probe.offset && probe.offset < end)
            {
                continue;
            }
            // One question per scrutinee position, in the order the
            // temporaries were emitted — which is the order of the
            // positions themselves.
            let temps: Vec<usize> = file
                .emit
                .scrutinee_temps
                .iter()
                .filter(|t| t.src == probe.offset)
                .map(|t| mapper::to_utf16(&file.emit.code, t.out))
                .collect();
            if temps.len() != probe.arity {
                continue;
            }
            for position in temps {
                query.tags.push(TagQuery {
                    module: file.module_path.clone(),
                    position,
                    covered: probe.covered.clone(),
                });
                probes.tags.push(MatchAnchor {
                    anchor: SourceAnchor {
                        source_path: file.source_path.clone(),
                        offset: probe.offset,
                        end: scrutinee_end(&file.source, probe.scrutinee_end),
                    },
                    body_open: probe.body_open,
                    body_close: probe.body_close,
                    tail: probe.tail,
                });
            }
        }

        // `val`: ttc finds the bindings and the mutations; which mutation
        // belongs to which binding is symbol identity, which is the
        // checker's to answer.
        let val = &file.val;
        for binding in &val.bindings {
            let Some(position) = anchor(&file.emit, binding.ident) else {
                continue;
            };
            query.symbols.push(SymbolQuery {
                module: file.module_path.clone(),
                position,
                binding: true,
            });
            probes.val_bindings.push(ValBindingAnchor {
                root: query.symbols.len() - 1,
                anchor: SourceAnchor {
                    source_path: file.source_path.clone(),
                    offset: binding.val_at,
                    end: binding.val_at + "val".len(),
                },
                modifier_end: binding.modifier_end,
            });
        }
        for mutation in &val.mutations {
            // A method call outside tt's mutator policy can never be
            // reported — the verdict needs the checker's `builtin` *and*
            // the policy name — so nothing is asked about it. The policy
            // itself lives at the verdict ([`crate::is_builtin_mutator_name`],
            // applied by the engine's report); skipping here is only the
            // observation that a question whose answer is settled is not
            // worth asking.
            if let Some((name, _)) = &mutation.method
                && !crate::is_builtin_mutator_name(name)
            {
                continue;
            }
            let Some(root) = anchor(&file.emit, mutation.root) else {
                continue;
            };
            // A method call needs a second question: is the method one of
            // TypeScript's own?
            let method = match &mutation.method {
                Some((_, at)) => match anchor(&file.emit, *at) {
                    Some(position) => {
                        query.symbols.push(SymbolQuery {
                            module: file.module_path.clone(),
                            position,
                            binding: false,
                        });
                        Some(query.symbols.len() - 1)
                    }
                    None => continue,
                },
                None => None,
            };
            query.symbols.push(SymbolQuery {
                module: file.module_path.clone(),
                position: root,
                binding: false,
            });
            probes.mutations.push(MutationAnchor {
                anchor: SourceAnchor {
                    source_path: file.source_path.clone(),
                    offset: mutation.root,
                    end: mutation.root + mutation.name.len(),
                },
                name: mutation.name.clone(),
                root: query.symbols.len() - 1,
                method,
                method_name: mutation.method.as_ref().map(|(name, _)| name.clone()),
            });
        }
        // The callee half: every declaration a call might name, as a node.
        // Which call names which declaration is symbol identity, so the
        // declaration identifiers are asked about alongside the calls.
        for function in &val.functions {
            let Some(position) = anchor(&file.emit, function.ident) else {
                continue;
            };
            query.symbols.push(SymbolQuery {
                module: file.module_path.clone(),
                position,
                binding: false,
            });
            probes.functions.push(FnAnchor {
                root: query.symbols.len() - 1,
                params: function.params.clone(),
            });
        }
        for pass in &val.passes {
            let Some(position) = anchor(&file.emit, pass.offset) else {
                continue;
            };
            let Some(callee_position) = anchor(&file.emit, pass.callee_at) else {
                continue;
            };
            query.symbols.push(SymbolQuery {
                module: file.module_path.clone(),
                position,
                binding: false,
            });
            let root = query.symbols.len() - 1;
            query.symbols.push(SymbolQuery {
                module: file.module_path.clone(),
                position: callee_position,
                binding: false,
            });
            probes.passes.push(PassAnchor {
                anchor: SourceAnchor {
                    source_path: file.source_path.clone(),
                    offset: pass.offset,
                    end: pass.offset + pass.name.len(),
                },
                name: pass.name.clone(),
                callee: pass.callee.clone(),
                root,
                callee_symbol: query.symbols.len() - 1,
                arg_index: pass.arg_index,
            });
        }
    }
    // A nested pattern narrows over the *payload*, whose type ttc may not
    // know — a type parameter, a hand-written union. The emitted condition
    // tests a receiver expression at exactly that type, and the emitter
    // recorded where; asking there names that column's alphabet for the
    // exhaustiveness algorithm.
    //
    // These ride in the same `tags` list (the question is the same: "which
    // `kind` values does this type allow?") with nothing covered, so the
    // answer is the whole alphabet. They are asked in a pass of their own,
    // **after** every file's match questions, so an answer's index splits
    // cleanly: below `probes.tags.len()` it is a match, at or above it a
    // payload. Interleaving them per file would misattribute every answer
    // from the second file on.
    for file in files {
        for probe in &file.payload_probes {
            let Some(temp) = file
                .emit
                .payload_temps
                .iter()
                .find(|t| t.src == probe.offset)
            else {
                continue;
            };
            query.tags.push(TagQuery {
                module: file.module_path.clone(),
                position: mapper::to_utf16(&file.emit.code, temp.out),
                covered: Vec::new(),
            });
            probes.payloads.push(PayloadAnchor {
                source_path: file.source_path.clone(),
                tag: probe.tag.clone(),
                field: probe.field.clone(),
            });
        }
    }

    // A source that failed tt projection still has to enter the candidate
    // set. The configured TypeScript program is the authority for whether
    // that path belongs through `files`/`include`/`exclude` or an import.
    // The placeholder carries no probes and no user code; it exists only so
    // the backend can return that membership decision in `projectModules`.
    query.modules.extend(blocked.iter().map(|file| Module {
        path: module_path_of(&file.source_path),
        text: "export {};\n".to_string(),
    }));
    query.syntax_blocked.extend(
        blocked
            .iter()
            .filter(|file| {
                file.diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code.restates_typescript_syntax())
            })
            .map(|file| module_path_of(&file.source_path)),
    );

    (query, probes)
}

/// The end of a `match`'s underlined range: the scrutinee's last byte,
/// extended over the whitespace and closing paren that follow it so the
/// span reads as the construct the user wrote — `match (shape)`.
fn scrutinee_end(source: &str, after_scrutinee: usize) -> usize {
    let rest = source.get(after_scrutinee..).unwrap_or_default();
    let closing = rest.trim_start();
    if closing.starts_with(')') {
        after_scrutinee + (rest.len() - closing.len()) + 1
    } else {
        after_scrutinee
    }
}

/// Where a question's answer is reported: a byte range in the `.tt`
/// source. `offset` is the position the CLI prints; `end` closes the range
/// an editor underlines (`offset` again when the anchor is a single point).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceAnchor {
    pub source_path: PathBuf,
    pub offset: usize,
    pub end: usize,
}

/// A match the checker was asked about: where its diagnostic is drawn, and
/// the braces of the body an arm-insertion edit writes between.
///
/// The typed pipeline reports a coverage hole from the checker's answer,
/// not from a parse, so the syntax the fix needs has to travel with the
/// question ([`crate::TagMatch`], [`crate::LiteralMatch`]) — which is what
/// lets both pipelines author the same edit (TASK-216).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MatchAnchor {
    /// Where the diagnostic is reported: the `match` keyword through the
    /// end of its head.
    pub anchor: SourceAnchor,
    /// Byte offset of the body's opening `{`.
    pub body_open: usize,
    /// Byte offset of the body's closing `}`.
    pub body_close: usize,
    /// Where the written arms end.
    pub tail: crate::ArmsTail,
}

/// One mutation, with the symbol questions that decide whether it is one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MutationAnchor {
    /// Where the diagnostic is reported.
    pub anchor: SourceAnchor,
    /// The root identifier's text, for the message.
    pub name: String,
    /// Index into [`Query::symbols`] for the root identifier: which binding
    /// this path is rooted at.
    pub root: usize,
    /// Index into [`Query::symbols`] for the method name, when the mutation
    /// is a method call. `None` for an assignment, an increment or a
    /// `delete`, which mutate on syntax alone.
    pub method: Option<usize>,
    /// The method's name, for the message.
    pub method_name: Option<String>,
}

/// One function declaration's symbol question, with the parameter list tt
/// read off it — the callee table's raw material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FnAnchor {
    /// Index into [`Query::symbols`] for the declared name's identifier.
    pub root: usize,
    pub params: Vec<crate::ValParam>,
}

/// One plain-path argument of a call to a name its file declares, with the
/// two symbol questions that decide whether it is a violation: the
/// argument's root (a `val` binding?) and the callee (which declaration?).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PassAnchor {
    /// Where the diagnostic is reported: the argument.
    pub anchor: SourceAnchor,
    /// The argument's root identifier, for the message.
    pub name: String,
    /// The called function's name.
    pub callee: String,
    /// Index into [`Query::symbols`] for the root identifier.
    pub root: usize,
    /// Index into [`Query::symbols`] for the callee identifier.
    pub callee_symbol: usize,
    /// Which argument this is, zero-based.
    pub arg_index: usize,
}

/// The `.tt`-side halves of a [`Query`], parallel to its own vectors.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Probes {
    pub literals: Vec<MatchAnchor>,
    pub tags: Vec<MatchAnchor>,
    /// One per nested pattern asked about, in the order the query lists
    /// them after [`Probes::tags`]: which file, and which
    /// `(constructor, field)` column the answer names the alphabet of.
    pub payloads: Vec<PayloadAnchor>,
    /// Every `val` binding's symbol question and declaration-side source span.
    pub val_bindings: Vec<ValBindingAnchor>,
    pub mutations: Vec<MutationAnchor>,
    /// The declarations a pass's callee may resolve to, project-wide.
    pub functions: Vec<FnAnchor>,
    pub passes: Vec<PassAnchor>,
    /// Explicit successful Result-return values, aligned with
    /// [`Query::result_shapes`].
    pub result_returns: Vec<SourceAnchor>,
}

/// One `val` declaration, paired with the symbol question for its binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValBindingAnchor {
    /// Index into [`Query::symbols`] for the binding identifier.
    pub root: usize,
    /// The `val` keyword itself, used as a rustc-style secondary label.
    pub anchor: SourceAnchor,
    /// End of the keyword and following horizontal whitespace, so removing
    /// the modifier leaves a clean TypeScript declaration.
    pub modifier_end: usize,
}

/// A payload column asked about: where it was written, and which
/// `(constructor, field)` it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PayloadAnchor {
    pub source_path: PathBuf,
    pub tag: String,
    pub field: String,
}

/// The UTF-16 offset in the emitted module a source byte landed at, or
/// `None` when it was not copied verbatim.
fn anchor(emit: &MappedEmit, source_byte: usize) -> Option<usize> {
    let out = mapper::to_output(&emit.mappings, source_byte)?;
    Some(mapper::to_utf16(&emit.code, out))
}

/// Where to ask about the type a `match` is over: the temporary the emitted
/// code binds the scrutinee to, found by the `match` keyword's own offset.
///
/// Not the scrutinee's text. `getTypeAtPosition` answers about the node at a
/// position, and for `match (getShape())` the node at the scrutinee's first
/// byte is `getShape` — a function, whose type has no `kind` property and no
/// literal constituents, so every exhaustiveness question came back silent.
/// The temporary is the scrutinee's *value*, and the type the checker gives
/// it is the narrowed one at the match. See [`crate::ScrutineeTemp`].
fn scrutinee_position(emit: &MappedEmit, keyword_offset: usize) -> Option<usize> {
    let temp = emit
        .scrutinee_temps
        .iter()
        .find(|temp| temp.src == keyword_offset)?;
    Some(mapper::to_utf16(&emit.code, temp.out))
}

/// Where a TypeScript diagnostic belongs in the `.tt` source, and whether
/// that position is exact.
///
/// A diagnostic on compiler-written glue is not the user's code, so its
/// position is approximate — the construct it was generated for — and the
/// message says so. By the error-layer contract it should not happen at
/// all: ttc's own output must not draw type errors.
/// The tt wording for a TypeScript diagnostic that landed on glue, with
/// the source span to report it over — `None` when the diagnostic is not
/// on glue, or when nothing in the whitelist covers it.
///
/// A diagnostic whose span *is* mapped is the user's own code and is never
/// translated: their type error is TypeScript's to phrase.
///
/// `declarations` is the file's declaration table — what lets the wording
/// name the variant case a structural type in the message lowers from. It is
/// the caller's because the table costs a parse of the file (and of what it
/// imports), which is worth doing once per file rather than once per
/// diagnostic.
#[cfg(test)]
fn translate_on_glue(
    file: &ProjectedDocument,
    diagnostic: &crate::typescript::backend::Diagnostic,
    declarations: &[crate::analysis::DeclaredVariant],
) -> Option<(crate::EmitAnchor, String)> {
    let anchor = glue_anchor(file, diagnostic.start)?;
    let said = super::semantics::translate(
        anchor.kind,
        diagnostic.code,
        &diagnostic.message,
        declarations,
    )?;
    Some((anchor, said))
}

/// The construct whose glue a diagnostic's start lands in — `None` when the
/// position is the user's own text (mapped) or belongs to no construct.
#[cfg(test)]
fn glue_anchor(file: &ProjectedDocument, utf16_start: usize) -> Option<crate::EmitAnchor> {
    let out = mapper::from_utf16(&file.emit.code, utf16_start);
    if mapper::to_source_inclusive(&file.emit.mappings, out).is_some() {
        return None;
    }
    file.emit.anchor_at(out).copied()
}

pub(crate) fn diagnostic_origin(
    file: &ProjectedDocument,
    utf16_start: usize,
    utf16_end: usize,
) -> Option<mapper::DiagnosticOrigin> {
    let (start, end) = (
        mapper::from_utf16(&file.emit.code, utf16_start),
        mapper::from_utf16(&file.emit.code, utf16_end),
    );
    if let Some(origin) = mapper::shared_binding_origin(&file.emit.shared_bindings, start, end) {
        return Some(origin);
    }
    mapper::diagnostic_origin(
        &file.emit.mappings,
        &file.emit.anchors,
        mapper::from_utf16(&file.emit.code, utf16_start),
        mapper::from_utf16(&file.emit.code, utf16_end),
        &file.emit.code,
        &file.source,
    )
}

fn source_extent(origin: mapper::DiagnosticOrigin) -> (usize, usize) {
    match origin {
        mapper::DiagnosticOrigin::Exact { start, end } => (start, end.max(start.saturating_add(1))),
        mapper::DiagnosticOrigin::Anchor(anchor) => (anchor.src, anchor.src_end),
        mapper::DiagnosticOrigin::Nearest { start } => (start, start.saturating_add(1)),
    }
}

/// Whether a checker diagnostic origin is already explained by a direct TT
/// cause. Display spans may be narrow; syntax-owner identity is what links
/// a cause to consequences emitted elsewhere in the same lowering. A tt
/// restatement of TypeScript's syntax verdict is no such cause: the
/// checker's diagnostic there is the verdict itself.
pub(crate) fn origin_intersects_tt_error(
    origin: mapper::DiagnosticOrigin,
    tt_diagnostics: &[crate::Diagnostic],
) -> bool {
    let (start, end) = source_extent(origin);
    tt_diagnostics
        .iter()
        .filter(|tt| !tt.code.restates_typescript_syntax())
        .any(|tt| {
            if let (mapper::DiagnosticOrigin::Anchor(anchor), Some(owner)) = (origin, tt.owner)
                && owner.start == anchor.src
                && owner.end == anchor.owner_end
            {
                return true;
            }
            let Some(tt_start) = tt.start else {
                return false;
            };
            let tt_end = tt.end.unwrap_or_else(|| tt_start.saturating_add(1));
            start < tt_end && tt_start < end
        })
}

pub(crate) fn restated(restatements: &[(usize, usize)], start: usize, end: usize) -> bool {
    restatements
        .iter()
        .any(|&(from, to)| from <= start && end <= to)
}

pub(crate) fn diagnostic_restates_source(
    file: &ProjectedDocument,
    diagnostic: &crate::typescript::backend::Diagnostic,
) -> bool {
    restated(
        &file.emit.restatements,
        mapper::from_utf16(&file.emit.code, diagnostic.start),
        mapper::from_utf16(&file.emit.code, diagnostic.end),
    )
}

pub(crate) fn diagnostic_intersects_recovery(
    file: &ProjectedDocument,
    diagnostic: &crate::typescript::backend::Diagnostic,
) -> bool {
    let Some(origin) = diagnostic_origin(file, diagnostic.start, diagnostic.end) else {
        return false;
    };
    let (start, end) = source_extent(origin);
    file.recovered
        .iter()
        .any(|&(recovery_start, recovery_end)| start < recovery_end && recovery_start < end)
}

/// Whether a checker diagnostic covers source already owned by a direct TT
/// cause. The mismatch span, when available, is the checker's more precise
/// statement of where the consequence originated; a property missing from
/// a value originates where that value is, so the receiver of the lookup
/// is owned as well.
pub(crate) fn diagnostic_intersects_tt_error(
    file: &ProjectedDocument,
    diagnostic: &crate::typescript::backend::Diagnostic,
) -> bool {
    let (diagnostic_start, diagnostic_end) = diagnostic
        .mismatch
        .as_ref()
        .map_or((diagnostic.start, diagnostic.end), |mismatch| {
            (mismatch.start, mismatch.end)
        });
    [
        Some((diagnostic_start, diagnostic_end)),
        diagnostic.receiver,
    ]
    .into_iter()
    .flatten()
    .filter_map(|(start, end)| diagnostic_origin(file, start, end))
    .any(|origin| origin_intersects_tt_error(origin, &file.tt_diagnostics))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typescript::backend::Diagnostic as TsDiagnostic;

    /// A diagnostic as the backend hands it over, at a byte of the emitted
    /// module found by searching for the glue in question.
    fn ts_at(file: &ProjectedDocument, needle: &str, code: u32, message: &str) -> TsDiagnostic {
        let at = file
            .emit
            .code
            .find(needle)
            .unwrap_or_else(|| panic!("no {needle:?} in emitted code"));
        TsDiagnostic {
            file: file.module_path.clone(),
            start: at,
            end: at + needle.len(),
            code,
            message: message.to_string(),
            mismatch: None,
            receiver: None,
            related: Vec::new(),
        }
    }

    /// The declaration table the report path hands the translation.
    fn declarations(file: &ProjectedDocument) -> Vec<crate::analysis::DeclaredVariant> {
        crate::pattern_analyses(&file.source, &[]).declarations
    }

    fn project(source: &str) -> ProjectedDocument {
        ProjectedDocument::project(Path::new("/p/src/a.tt"), source.to_string()).expect("projects")
    }

    #[test]
    fn parser_error_nodes_recover_only_their_own_source_ranges() {
        let source = "const broken = ready ? 1 : 2 |> f;\n\
            const independent: string = 1;\n\
            const optional = value |> ?.member + 1;\n\
            const malformed = match value { Missing => 0 };\n";
        let file = project(source);
        assert_eq!(file.recovered.len(), 3, "{:#?}", file.recovered);
        assert!(file.emit.code.contains("const independent: string = 1;"));
        let independent = source.find("independent").unwrap();
        assert!(
            file.recovered
                .iter()
                .all(|&(start, end)| independent < start || independent >= end)
        );
    }

    #[test]
    fn a_malformed_match_beside_a_variant_recovers_before_codegen() {
        let source = "variant Shape { Circle(r: number), Square(s: number) }\n\
            export function area(shape: Shape): number {\n\
              return match shape { Circle(r) => r, Square(s) => s };\n\
            }\n";
        let file = project(source);
        assert_eq!(file.recovered.len(), 1, "{:#?}", file.recovered);
        assert!(
            file.emit.code.contains("type Shape ="),
            "{}",
            file.emit.code
        );
        assert!(
            file.tt_diagnostics
                .iter()
                .any(|d| d.code == crate::DiagnosticCode::MalformedMatch),
            "{:#?}",
            file.tt_diagnostics
        );
    }

    #[test]
    fn a_type_error_on_a_constructs_glue_is_reported_in_tts_words() {
        // `try` on a non-Result: TypeScript reaches for `.kind` on a number
        // and says so about code the user never wrote.
        let file = project("function f() {\n  const a = try plain();\n  return a;\n}\n");
        let diagnostic = ts_at(
            &file,
            "\"value\" in $tt_t0",
            2339,
            "Property 'kind' does not exist on type 'number'.",
        );
        let (anchor, said) =
            translate_on_glue(&file, &diagnostic, &declarations(&file)).expect("translated");
        // The propagation itself is the span — not the whole declaration,
        // and not one character of it.
        assert_eq!(&file.source[anchor.src..anchor.src_end], "try plain()");
        assert!(said.starts_with("`try` needs a `Result`"), "{said}");
        // The original rides along — a translation the user can check.
        assert!(said.contains("ts2339: Property 'kind'"), "{said}");
    }

    #[test]
    fn a_type_error_on_the_users_own_code_is_left_to_typescript() {
        let file = project("function f() {\n  const a = try plain();\n  return a;\n}\n");
        // `plain()` is copied from the source, so it is mapped — the user's
        // own text, and their type error to read as TypeScript phrased it.
        let diagnostic = ts_at(&file, "plain()", 2554, "Expected 1 arguments, but got 0.");
        assert!(translate_on_glue(&file, &diagnostic, &declarations(&file)).is_none());
    }

    #[test]
    fn an_unrecognized_code_on_glue_is_not_guessed_at() {
        let file = project("function f() {\n  const a = try plain();\n  return a;\n}\n");
        let diagnostic = ts_at(
            &file,
            "\"value\" in $tt_t0",
            2739,
            "Type is missing properties.",
        );
        assert!(translate_on_glue(&file, &diagnostic, &declarations(&file)).is_none());
    }

    #[test]
    fn the_innermost_construct_owns_its_glue() {
        let file = project(
            "variant E { A(x: number), B }\nfunction f() {\n  const a = try wrap(match (e) { A(x) => x, B => 0 });\n}\n",
        );
        let diagnostic = ts_at(
            &file,
            "$tt_m.kind",
            2339,
            "Property 'kind' does not exist on type 'Plain'.",
        );
        let (anchor, said) =
            translate_on_glue(&file, &diagnostic, &declarations(&file)).expect("translated");
        assert_eq!(&file.source[anchor.src..anchor.src_end], "match (e)");
        assert!(said.starts_with("match on a tag pattern"), "{said}");
    }

    #[test]
    fn a_recoverable_tt_error_does_not_block_the_projection() {
        // TASK-117 symptom 3: a duplicate arm used to fail the projection
        // (`Blocked`) and silence the file's typed diagnostics wholesale.
        // Now the file lowers, and the error rides along for the report.
        let file = project(
            "variant E { A(x: number), B }\n\
             const v = match (E.A(1)) { A(x) => x, A(x) => 0, B => 1 };\n",
        );
        assert!(file.emit.code.contains("switch ($tt_m.kind)"));
        assert_eq!(file.tt_diagnostics.len(), 1);
        assert_eq!(
            file.tt_diagnostics[0].code,
            crate::DiagnosticCode::MatchDuplicateArm
        );
    }

    #[test]
    fn resolution_diagnostics_ride_along_on_the_typed_path_too() {
        let file = project(
            "variant Shape { Circle(r: number), Empty }\n\
             const v = match (s) { Circel(r) => r, Empty => 0 };\n",
        );
        assert_eq!(file.tt_diagnostics.len(), 1);
        assert_eq!(
            file.tt_diagnostics[0].code,
            crate::DiagnosticCode::UnknownCase
        );
    }

    #[test]
    fn text_that_cannot_lower_gets_an_engine_only_error_node() {
        let file = project("const x = a ? 1 : 2 |> f;\n");
        assert_eq!(file.recovered, [(18, 24)]);
        assert!(
            file.emit.code.contains("const x = a ? 1 : 0     ;"),
            "{}",
            file.emit.code
        );
        assert_eq!(file.tt_diagnostics.len(), 1);
        assert_eq!(
            file.tt_diagnostics[0].code,
            crate::DiagnosticCode::StrayPipe
        );
    }

    #[test]
    fn a_lowered_module_is_named_so_that_an_tt_specifier_resolves_to_it() {
        // `import "./state.tt"` resolves to `state.tt.ts` — and the
        // declaration emitted for it lands on `state.tt.d.ts`, the sidecar
        // the same specifier resolves to without a compiler.
        assert_eq!(
            module_path_of(Path::new("/p/src/state.tt")),
            PathBuf::from("/p/src/state.tt.ts"),
        );
    }
}
