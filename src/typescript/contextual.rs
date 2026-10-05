//! Contextual type facts applied to explicit codegen value-storage sites.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::backend::{ContextualSlotQuery, Failure, FailureKind, Module, Query, TypeScriptBackend};
use super::mapper;
use crate::MappedEmit;
use crate::codegen::contextual::{SlotRefinement, refine};

/// Settles every value slot of `modules` and leaves each module refined.
///
/// Each successful round annotates at least one previously unannotated
/// slot. Re-querying the updated graph lets outer contexts reach nested
/// slots. Once no slot has a further contextual type, every slot still
/// without one is detached: its source position has no contextual type, so
/// its values are written through `const`s of their own, and only then are
/// joins inferred. A detached slot a later round finds a contextual type
/// for is written directly again, under that type.
pub(crate) fn materialize(
    backend: &impl TypeScriptBackend,
    config: Option<&Path>,
    root: &Path,
    modules: &mut [(PathBuf, MappedEmit)],
    support: &[Module],
    sources: &[PathBuf],
    roots: &[PathBuf],
) -> Result<Vec<Vec<SlotRefinement>>, Failure> {
    let bases: Vec<MappedEmit> = modules.iter().map(|(_, emit)| emit.clone()).collect();
    let mut slots: Vec<Vec<SlotRefinement>> = bases
        .iter()
        .map(|emit| vec![SlotRefinement::default(); emit.contextual_slots.len()])
        .collect();
    let mut infer_joins = false;
    loop {
        let mut query = Query {
            contextual_only: true,
            infer_join_types: infer_joins,
            sources: sources.to_vec(),
            roots: roots.to_vec(),
            modules: support.to_vec(),
            ..Query::default()
        };
        let mut sites = Vec::new();
        let mut settled = Vec::new();
        for (module_index, ((path, emit), base)) in modules.iter_mut().zip(&bases).enumerate() {
            *emit = base.clone();
            let kind = crate::SourceKind::from_path(path).unwrap_or_default();
            let refined = refine(emit, kind, &slots[module_index]);
            query.modules.push(Module {
                path: path.clone(),
                text: emit.code.clone(),
            });
            let at = |position: usize, settled: bool, operand: bool, asserted: bool| {
                ContextualSlotQuery {
                    module: path.clone(),
                    declaration_end: mapper::to_utf16(&emit.code, position),
                    settled,
                    operand,
                    asserted,
                }
            };
            for (index, (&position, slot)) in refined
                .declarations
                .iter()
                .zip(&slots[module_index])
                .enumerate()
            {
                let declared = base.contextual_slots[index];
                let asserted = slot.provisional
                    || base
                        .asserted_slots
                        .iter()
                        .any(|&(slot_end, _)| slot_end == declared);
                if slot.cleared || (slot.annotation.is_some() && !slot.provisional) {
                    settled.push(at(position, true, false, false));
                } else {
                    sites.push((module_index, index));
                    let operand = base.operand_slots.contains(&declared);
                    query
                        .contextual_slots
                        .push(at(position, false, operand, asserted));
                }
            }
            settled.extend(
                refined
                    .locals
                    .iter()
                    .map(|&position| at(position, true, false, false)),
            );
        }
        // After every slot an answer can name, so an answer's index is its
        // site's.
        query.contextual_slots.extend(settled);
        if sites.is_empty() {
            return Ok(slots);
        }
        crate::work::tick("contextual checker asks");
        let answers = backend.ask(config, root, &query)?;
        if answers.contextual_slots.is_empty() {
            if infer_joins {
                return Ok(slots);
            }
            for &(module, index) in &sites {
                let asserted = slots[module][index].provisional
                    || bases[module]
                        .asserted_slots
                        .iter()
                        .any(|&(slot_end, _)| slot_end == bases[module].contextual_slots[index]);
                if !asserted {
                    slots[module][index].detached = true;
                }
            }
            infer_joins = true;
            continue;
        }
        // New inferred storage can expose further contexts; propagate those
        // before inferring any more joins.
        infer_joins = false;
        for answer in answers.contextual_slots {
            let &(module, index) = sites.get(answer.index).ok_or_else(|| {
                Failure::internal("contextual answer names an unknown value slot")
            })?;
            let slot = &mut slots[module][index];
            if slot.annotation.is_some() && !slot.provisional {
                return Err(Failure::internal("contextual answer repeats a value slot"));
            }
            slot.cleared = answer.annotation.is_none();
            slot.annotation = answer.annotation;
            slot.provisional = answer.provisional;
            if !answer.inferred {
                slot.detached = false;
            }
        }
    }
}

/// The standard-library package, served from the compiler's own modules.
///
/// ttc holds these sources; a project only ever gets a *copy* of them, and
/// materializing one is the build's job, not a precondition for typing the
/// storage this pass annotates. TypeScript resolves `@tt/std` through the
/// file system, so the package has to be visible — the layered filesystem
/// the host runs on makes a served module visible exactly that way, which
/// is why nothing is written to disk here.
///
/// A package the project already has on disk is left alone: a project that
/// manages its own copy keeps it, and shadowing it would type this file
/// against sources the project does not use.
fn std_support(root: &Path) -> Vec<Module> {
    crate::StdPackage::ALL
        .into_iter()
        .filter(|package| !package_exists(root, package.name()))
        .flat_map(|package| {
            let directory = package.directory(root);
            package.files().into_iter().map(move |(name, text)| Module {
                path: directory.join(name),
                text,
            })
        })
        .collect()
}

fn package_exists(root: &Path, name: &str) -> bool {
    root.ancestors()
        .any(|ancestor| ancestor.join("node_modules").join(name).exists())
}

/// Standalone project-input errors are distinct from backend availability.
#[derive(Debug)]
pub(crate) enum StandaloneFailure {
    Input(String),
    Backend(Failure),
}

impl From<Failure> for StandaloneFailure {
    fn from(failure: Failure) -> Self {
        Self::Backend(failure)
    }
}

impl std::fmt::Display for StandaloneFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(message) => f.write_str(message),
            Self::Backend(failure) => f.write_str(&failure.message),
        }
    }
}

/// Compile a file against its project graph, or an unnamed buffer against
/// the caller's working directory. Project consumers use `materialize` with
/// their authoritative in-memory overlays instead.
pub(crate) fn standalone(
    mut emit: MappedEmit,
    source: &str,
    options: &crate::Options<'_>,
) -> Result<MappedEmit, StandaloneFailure> {
    if emit.contextual_slots.is_empty() {
        return Ok(emit);
    }
    let cwd =
        std::env::current_dir().map_err(|error| StandaloneFailure::Input(error.to_string()))?;
    let file = options
        .filename
        .map(|name| cwd.join(name))
        .filter(|path| path.is_file())
        .map(|path| {
            path.canonicalize()
                .map_err(|error| StandaloneFailure::Input(error.to_string()))
        })
        .transpose()?;
    let config = file
        .as_ref()
        .and_then(|file| file.parent())
        .and_then(|parent| {
            parent
                .ancestors()
                .map(|dir| dir.join("tsconfig.json"))
                .find(|path| path.is_file())
        });
    let root = config
        .as_ref()
        .and_then(|path| path.parent())
        .or_else(|| file.as_ref().and_then(|path| path.parent()))
        .unwrap_or(&cwd)
        .to_path_buf();
    let inferred_config = root.join(format!(".tt-contextual-{}.json", std::process::id()));
    let configuration = config.clone().unwrap_or_else(|| inferred_config.clone());
    let session = ProjectSession::of(&root, options.node);
    // Availability is decided before reading project inputs: a backend is
    // available once its toolchain resolves and its host is running. From
    // then on, project and backend failures must reach the caller unchanged.
    let available = {
        let mut session = ProjectSession::lock(&session);
        if session.backend.is_none() {
            let Ok(backend) =
                super::native::NativeBackend::new(options.node.map(Path::to_path_buf), &cwd)
            else {
                return Ok(emit);
            };
            session.backend = Some(backend);
        }
        let backend = session.backend.as_ref().expect("backend initialized above");
        match backend.open(Some(&configuration), &root) {
            Ok(()) => true,
            Err(failure) if failure.kind == FailureKind::Unavailable => false,
            Err(failure) => return Err(failure.into()),
        }
    };
    if !available {
        return Ok(emit);
    }
    let path = file
        .as_ref()
        .map(|path| module_path(path))
        .unwrap_or_else(|| {
            root.join(if options.source_kind == crate::SourceKind::Tsx {
                "__tt_contextual_input.tsx"
            } else {
                "__tt_contextual_input.ts"
            })
        });
    let projection_options = crate::Options {
        defer_to_checker: true,
        rewrite_imports: crate::ImportRewrite::Off,
        // Analysis resolves compiler-owned modules in the source project.
        // Adapter output paths need not exist until after compilation.
        std_imports: crate::StdImports::default(),
        ..options.clone()
    };
    let analysis = crate::compile_projection_report(source, &projection_options)
        .emit
        .ok_or_else(|| {
            Failure::internal("contextual projection lost a successfully lowered module")
        })?;
    if analysis.contextual_slots.len() != emit.contextual_slots.len() {
        return Err(Failure::internal("contextual projection changed value slot identity").into());
    }
    // The checker admits modules through the user's configuration and imports;
    // the scan only makes tt projections available to its filesystem. It is
    // read outside the session so parallel callers read in parallel.
    let mut candidates = Vec::new();
    if file.is_some() {
        // A failed walk is not a complete snapshot: its prefix can omit
        // readable dependencies after the failing entry.
        let mut paths = Vec::new();
        crate::engine::collect_sources(&root, false, &mut paths)
            .map_err(|error| StandaloneFailure::Input(error.to_string()))?;
        for candidate in paths {
            let source = std::fs::read_to_string(&candidate).map_err(|error| {
                StandaloneFailure::Input(format!("{}: {error}", candidate.display()))
            })?;
            candidates.push((candidate, source));
        }
    }
    let types = {
        let mut session = ProjectSession::lock(&session);
        let ProjectSession { backend, reuse } = &mut *session;
        let backend = backend.as_ref().expect("backend initialized above");
        if file.is_some() {
            reuse.reconcile(candidates);
        }
        // An unnamed or unconfigured source must retain nullability when its
        // generated annotations are later checked in a strict project.
        let mut support = std_support(&root);
        support.extend(if config.is_none() {
            vec![Module { path: inferred_config.clone(), text: serde_json::json!({
                "compilerOptions": { "strict": true, "target": "esnext", "module": "preserve", "moduleResolution": "bundler", "jsx": "preserve", "skipLibCheck": true, "noEmit": true },
                "files": [path]
            }).to_string() }]
        } else { Vec::new() });
        let configuration = configuration.as_path();
        let asked = Materialization {
            configuration: configuration.to_path_buf(),
            root: root.clone(),
            support,
            version: reuse.version,
        };
        // Over an unchanged graph the answers were computed with every
        // module's projection except the one requested then, which was that
        // file's own analysis; they stand for this request when its module
        // is the one asked about now.
        let reused = reuse.answers.as_ref().and_then(|answered| {
            let index = answered
                .modules
                .binary_search_by(|(module, _)| module.cmp(&path))
                .ok()?;
            let (requested, requested_source) = &answered.requested;
            let projected = |module: &MappedEmit| {
                requested_source
                    .as_ref()
                    .and_then(|source| reuse.projections.get(source))
                    .and_then(|(_, emit)| emit.as_ref())
                    == Some(module)
            };
            (answered.asked == asked
                && answered.modules[index].1 == analysis
                && (index == *requested || projected(&answered.modules[*requested].1))
                && backend.current_generation(Some(configuration), &root)
                    == Some(answered.generation))
            .then(|| answered.types[index].clone())
        });
        match reused {
            Some(types) => types,
            None => {
                let mut modules = vec![(path.clone(), analysis)];
                if file.is_some() {
                    modules.extend(reuse.projections.iter().filter_map(
                        |(candidate, (_, emit))| {
                            let module = module_path(candidate);
                            (module != path).then(|| emit.clone().map(|emit| (module, emit)))?
                        },
                    ));
                }
                modules.sort_by(|left, right| left.0.cmp(&right.0));
                let asked_modules = modules.clone();
                backend.observe_generations();
                let types = materialize(
                    backend,
                    Some(configuration),
                    &root,
                    &mut modules,
                    &asked.support,
                    &[],
                    &[],
                )?;
                let requested = modules
                    .iter()
                    .position(|(module, _)| *module == path)
                    .ok_or_else(|| {
                        Failure::internal("contextual projection lost the requested module")
                    })?;
                let requested_types = types[requested].clone();
                reuse.answers = backend.stable_generation().map(|generation| Answered {
                    asked,
                    modules: asked_modules,
                    requested: (requested, file.clone()),
                    generation,
                    types,
                });
                requested_types
            }
        }
    };
    let mut refinements = Vec::with_capacity(types.len());
    for slot in types {
        let annotation = match slot.annotation {
            // Reuse the compiler's import-specifier model for synthesized
            // import types, exactly as for authored import types.
            Some(annotation) => {
                let wrapper = format!("type __tt_context = {annotation};");
                let rewritten = crate::compile(
                    &wrapper,
                    &crate::Options {
                        rewrite_imports: options.rewrite_imports,
                        std_imports: options.std_imports,
                        defer_to_checker: true,
                        ..crate::Options::default()
                    },
                )
                .map_err(|error| Failure::internal(error.to_string()))?;
                Some(rewritten["type __tt_context = ".len()..rewritten.len() - 1].to_owned())
            }
            None => None,
        };
        refinements.push(SlotRefinement {
            annotation,
            detached: slot.detached,
            provisional: slot.provisional,
            cleared: slot.cleared,
        });
    }
    refine(&mut emit, options.source_kind, &refinements);
    Ok(emit)
}

/// What this process knows about one project for [`standalone`]: the
/// backend session every caller asks about it, and that session's last
/// answers. Parallel workers compiling one project share it, so the project
/// is opened and planned once rather than once per worker.
#[derive(Default)]
struct ProjectSession {
    backend: Option<super::native::NativeBackend>,
    reuse: Reuse,
}

impl ProjectSession {
    /// The session for the project at `root`. The most recently used
    /// sessions are kept, one per core — as many as one per worker thread
    /// would hold — and an evicted one ends its host once no caller holds it.
    fn of(root: &Path, node: Option<&Path>) -> Arc<Mutex<ProjectSession>> {
        type Sessions = Vec<((PathBuf, Option<PathBuf>), Arc<Mutex<ProjectSession>>)>;
        static SESSIONS: Mutex<Sessions> = Mutex::new(Vec::new());
        let mut sessions = SESSIONS.lock().unwrap_or_else(PoisonError::into_inner);
        let key = (root.to_path_buf(), node.map(Path::to_path_buf));
        let session = match sessions.iter().position(|(project, _)| *project == key) {
            Some(index) => sessions.remove(index).1,
            None => Arc::default(),
        };
        sessions.push((key, session.clone()));
        let kept = std::thread::available_parallelism().map_or(1, usize::from);
        let evicted = sessions.len().saturating_sub(kept);
        sessions.drain(..evicted);
        session
    }

    /// A caller that panicked mid-exchange may have left the host's protocol
    /// half-read, so a poisoned session starts over.
    fn lock(session: &Mutex<ProjectSession>) -> MutexGuard<'_, ProjectSession> {
        session.lock().unwrap_or_else(|poisoned| {
            session.clear_poison();
            let mut session = poisoned.into_inner();
            *session = ProjectSession::default();
            session
        })
    }
}

type SlotTypes = Vec<Vec<SlotRefinement>>;

#[derive(Default)]
struct Reuse {
    /// Every tt source of the project as last read, and its projection.
    projections: HashMap<PathBuf, (String, Option<MappedEmit>)>,
    /// Advances whenever `projections` changes.
    version: u64,
    answers: Option<Answered>,
}

impl Reuse {
    /// Brings the projections up to the sources just read, re-projecting
    /// only what changed.
    fn reconcile(&mut self, sources: Vec<(PathBuf, String)>) {
        let mut previous = std::mem::take(&mut self.projections);
        let mut changed = false;
        for (candidate, source) in sources {
            let emit = match previous.remove(&candidate) {
                Some((projected, emit)) if projected == source => emit,
                _ => {
                    changed = true;
                    crate::work::tick("contextual projections");
                    crate::ice::catching(|| {
                        crate::ice::working_on(&candidate, || {
                            crate::ice::panic_for_test(&format!(
                                "projection:{}",
                                candidate.file_name().unwrap_or_default().to_string_lossy()
                            ));
                            crate::compile_projection_report(
                                &source,
                                &crate::Options {
                                    source_kind: crate::SourceKind::from_path(&candidate)
                                        .unwrap_or_default(),
                                    defer_to_checker: true,
                                    rewrite_imports: crate::ImportRewrite::Off,
                                    ..crate::Options::default()
                                },
                            )
                            .emit
                        })
                    })
                    .unwrap_or(None)
                }
            };
            self.projections.insert(candidate, (source, emit));
        }
        if changed || !previous.is_empty() {
            self.version += 1;
        }
    }
}

/// One materialization: what it was asked about, and what it answered for
/// each of its modules.
struct Answered {
    asked: Materialization,
    /// The modules as sent, sorted by path.
    modules: Vec<(PathBuf, MappedEmit)>,
    /// The requested module's index, and its source when it has one.
    requested: (usize, Option<PathBuf>),
    generation: (u64, u64),
    types: SlotTypes,
}

#[derive(PartialEq)]
struct Materialization {
    configuration: PathBuf,
    root: PathBuf,
    support: Vec<Module>,
    version: u64,
}

fn module_path(path: &Path) -> PathBuf {
    let extension = if crate::SourceKind::from_path(path) == Some(crate::SourceKind::Tsx) {
        "tsx"
    } else {
        "ts"
    };
    PathBuf::from(format!("{}.{extension}", path.display()))
}
