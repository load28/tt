//! Contextual type facts applied to explicit codegen value-storage sites.

use std::path::{Path, PathBuf};

use super::backend::{ContextualSlotQuery, Failure, Module, Query, TypeScriptBackend};
use super::mapper;
use crate::MappedEmit;
use crate::codegen::contextual::insert_annotations;

/// Each successful round annotates at least one previously unannotated slot.
/// Re-querying the updated graph lets outer contexts reach nested slots.
pub(crate) fn materialize(
    backend: &impl TypeScriptBackend,
    config: Option<&Path>,
    root: &Path,
    modules: &mut [(PathBuf, MappedEmit)],
    support: &[Module],
    sources: &[PathBuf],
) -> Result<Vec<Vec<Option<String>>>, Failure> {
    let mut types: Vec<Vec<Option<String>>> = modules
        .iter()
        .map(|(_, emit)| vec![None; emit.contextual_slots.len()])
        .collect();
    let mut origins: Vec<Vec<usize>> = modules
        .iter()
        .map(|(_, emit)| (0..emit.contextual_slots.len()).collect())
        .collect();
    loop {
        let mut query = Query {
            contextual_only: true,
            sources: sources.to_vec(),
            modules: support.to_vec(),
            ..Query::default()
        };
        let mut sites = Vec::new();
        for (module_index, (path, emit)) in modules.iter().enumerate() {
            query.modules.push(Module {
                path: path.clone(),
                text: emit.code.clone(),
            });
            for (index, &position) in emit.contextual_slots.iter().enumerate() {
                sites.push((module_index, position, origins[module_index][index]));
                query.contextual_slots.push(ContextualSlotQuery {
                    module: path.clone(),
                    declaration_end: mapper::to_utf16(&emit.code, position),
                });
            }
        }
        if sites.is_empty() {
            return Ok(types);
        }
        let answers = backend.ask(config, root, &query)?;
        if answers.contextual_slots.is_empty() {
            return Ok(types);
        }
        let mut edits = vec![Vec::new(); modules.len()];
        for answer in answers.contextual_slots {
            let &(module, position, origin) = sites.get(answer.index).ok_or_else(|| {
                Failure::internal("contextual answer names an unknown value slot")
            })?;
            if edits[module]
                .iter()
                .any(|(previous, _)| *previous == position)
            {
                return Err(Failure::internal("contextual answer repeats a value slot"));
            }
            types[module][origin] = Some(answer.annotation.clone());
            edits[module].push((position, answer.annotation));
        }
        for (module, ((_, emit), mut edits)) in modules.iter_mut().zip(edits).enumerate() {
            origins[module].retain(|origin| types[module][*origin].is_none());
            edits.sort_by_key(|edit| edit.0);
            insert_annotations(emit, &edits);
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
) -> Result<MappedEmit, Failure> {
    if emit.contextual_slots.is_empty() {
        return Ok(emit);
    }
    thread_local! {
        static BACKEND: std::cell::RefCell<Option<(PathBuf, super::native::NativeBackend)>> = const { std::cell::RefCell::new(None) };
    }
    let cwd = std::env::current_dir().map_err(|error| Failure::unavailable(error.to_string()))?;
    let file = options
        .filename
        .map(|name| cwd.join(name))
        .filter(|path| path.is_file())
        .map(|path| {
            path.canonicalize()
                .map_err(|error| Failure::unavailable(error.to_string()))
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
        ..options.clone()
    };
    let analysis = crate::compile_projection_report(source, &projection_options)
        .emit
        .ok_or_else(|| {
            Failure::internal("contextual projection lost a successfully lowered module")
        })?;
    if analysis.contextual_slots.len() != emit.contextual_slots.len() {
        return Err(Failure::internal(
            "contextual projection changed value slot identity",
        ));
    }
    let mut modules = vec![(path, analysis)];
    // The checker admits modules through the user's configuration and imports;
    // the scan only makes tt projections available to its filesystem.
    if let Some(file) = &file {
        let mut candidates = Vec::new();
        crate::engine::collect_sources(&root, false, &mut candidates)
            .map_err(|error| Failure::unavailable(error.to_string()))?;
        for candidate in candidates {
            if candidate == *file {
                continue;
            }
            let source = std::fs::read_to_string(&candidate)
                .map_err(|error| Failure::unavailable(error.to_string()))?;
            let report = crate::compile_projection_report(
                &source,
                &crate::Options {
                    source_kind: crate::SourceKind::from_path(&candidate).unwrap_or_default(),
                    defer_to_checker: true,
                    rewrite_imports: crate::ImportRewrite::Off,
                    ..crate::Options::default()
                },
            );
            if let Some(emit) = report.emit {
                modules.push((module_path(&candidate), emit));
            }
        }
    }
    BACKEND.with(|cell| {
        let mut state = cell.borrow_mut();
        if state.as_ref().is_none_or(|(previous, _)| previous != &root) {
            *state = Some((
                root.clone(),
                super::native::NativeBackend::new(None, &cwd).map_err(Failure::unavailable)?,
            ));
        }
        let (_, backend) = state.as_ref().expect("backend initialized above");
        // An unnamed or unconfigured source must retain nullability when its
        // generated annotations are later checked in a strict project.
        let inferred_config = root.join(format!(".tt-contextual-{}.json", std::process::id()));
        let support = if config.is_none() {
            vec![Module { path: inferred_config.clone(), text: serde_json::json!({
                "compilerOptions": { "strict": true, "target": "esnext", "module": "preserve", "moduleResolution": "bundler", "jsx": "preserve", "skipLibCheck": true, "noEmit": true },
                "files": [modules[0].0]
            }).to_string() }]
        } else { Vec::new() };
        let configuration = config.as_deref().unwrap_or(&inferred_config);
        let types = materialize(backend, Some(configuration), &root, &mut modules, &support, &[])?;
        let mut edits = Vec::new();
        for (position, annotation) in emit.contextual_slots.iter().copied().zip(&types[0]) {
            let Some(annotation) = annotation else {
                continue;
            };
            // Reuse the compiler's import-specifier model for synthesized
            // import types, exactly as for authored import types.
            let wrapper = format!("type __tt_context = {annotation};");
            let rewritten = crate::compile(
                &wrapper,
                &crate::Options {
                    rewrite_imports: options.rewrite_imports,
                    defer_to_checker: true,
                    ..crate::Options::default()
                },
            )
            .map_err(|error| Failure::internal(error.to_string()))?;
            let annotation = &rewritten["type __tt_context = ".len()..rewritten.len() - 1];
            edits.push((position, annotation.to_owned()));
        }
        insert_annotations(&mut emit, &edits);
        Ok(emit)
    })
}

fn module_path(path: &Path) -> PathBuf {
    let extension = if crate::SourceKind::from_path(path) == Some(crate::SourceKind::Tsx) {
        "tsx"
    } else {
        "ts"
    };
    PathBuf::from(format!("{}.{extension}", path.display()))
}
