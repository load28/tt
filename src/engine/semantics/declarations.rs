//! Match declaration and external-variant projection.

use super::*;

/// Matches the compiler's emitted declarations back to the snapshot's files.
/// Only requested files are kept; the rest only support graph resolution.
pub(crate) fn match_declarations(
    snapshot: &Snapshot,
    answers: &Answers,
    root: &std::path::Path,
    requested: &HashSet<PathBuf>,
) -> Declarations {
    let mut out = Declarations::default();
    // The standard library's own declarations, so a consumer running plain
    // tsc can map every `@tt/std` entry to them. They are project modules
    // like any other, but have no `.tt` sources to sit beside.
    for declaration in &answers.declarations {
        if let Some(module) = crate::StdModule::ALL.into_iter().find(|module| {
            declaration.path
                == root
                    .join(projection::std_module_path(*module))
                    .with_extension("d.ts")
        }) {
            out.std.push(StdDeclaration {
                module,
                text: declaration.text.clone(),
            });
            continue;
        }
        let Some(file) = snapshot
            .files()
            .iter()
            .find(|f| projection::declaration_path_of(f) == declaration.path)
            .filter(|f| requested.contains(&f.source_path))
        else {
            continue;
        };
        out.modules.push(ModuleDeclaration {
            file: file.clone(),
            text: declaration.text.clone(),
        });
    }
    out
}

/// The variant declarations one file's direct `.tt` imports bring into scope,
/// preferring the snapshot's own text for a file it holds — the same
/// 1-hop collection every other surface does, so an imported variant is known
/// under the name the import gave it.
/// The imported declarations in `file`'s scope, read from the snapshot's
/// cached per-file symbols where the import target is in the snapshot —
/// a target that did not change is never re-parsed — and from disk
/// otherwise.
pub(crate) fn externs_of(
    snapshot: &Snapshot,
    file: &ProjectedDocument,
) -> Vec<crate::VariantSymbol> {
    super::super::language::externs_from(&file.source_path, file.tt_imports(), &|target| {
        snapshot
            .files()
            .iter()
            .find(|f| f.source_path == target)
            .map(|f| {
                f.variant_symbols()
                    .iter()
                    .filter(|d| d.exported)
                    .cloned()
                    .collect()
            })
            .or_else(|| {
                snapshot
                    .blocked()
                    .iter()
                    .find(|f| f.source_path == target)
                    .map(|f| {
                        f.variant_symbols()
                            .iter()
                            .filter(|d| d.exported)
                            .cloned()
                            .collect()
                    })
            })
            .or_else(|| {
                let text = std::fs::read_to_string(target).ok()?;
                Some(
                    crate::variant_symbols_with_kind(
                        &text,
                        crate::SourceKind::from_path(target).unwrap_or_default(),
                    )
                    .into_iter()
                    .filter(|d| d.exported)
                    .collect(),
                )
            })
    })
}

/// A covered literal as it reads in a message.
pub(super) fn display_literal(literal: &crate::Literal) -> String {
    match literal {
        crate::Literal::String(s) => format!("{s:?}"),
        crate::Literal::Number(n) => n.to_string(),
        crate::Literal::BigInt(d) => format!("{d}n"),
        crate::Literal::Boolean(b) => b.to_string(),
    }
}
